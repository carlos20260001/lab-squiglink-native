//! The phone's own files.
//!
//! The system picker on a phone hands its answer back through the activity,
//! which the activity the game runs in does not pass on; so the files are
//! listed from the system's own file table instead, and the one that is
//! wanted is read from there.
//!
//! Everything here is the system's own API, read through the JNI.

use std::path::PathBuf;

use jni::objects::{JObject, JValue};

/// A file the device holds.
#[derive(Clone, Debug)]
pub struct File {
    /// The file's name, as the system shows it.
    pub name: String,
    /// Where the system keeps it.
    pub uri: String,
    /// Its kind, as the system says it.
    pub mime: String,
    /// How large it is, in bytes.
    pub size: i64,
}

/// The files the device holds of the given kinds.
///
/// The kinds are the system's own media types, as the system's own tables
/// name them.
pub fn list(mime_types: &[&str]) -> Vec<File> {
    let Some(mut guard) = thread() else {
        return Vec::new();
    };
    let env = &mut *guard;
    let context = ndk_context::android_context();
    let activity = unsafe { JObject::from_raw(context.context() as jni::sys::jobject) };
    let Ok(resolver) = env
        .call_method(
            &activity,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .and_then(|value| value.l())
    else {
        return Vec::new();
    };
    let Ok(text) = env.new_string("content://media/external/file") else {
        return Vec::new();
    };
    let Ok(uri) = env.new_object(
        "android/net/Uri",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&text)],
    ) else {
        return Vec::new();
    };
    let Ok(projection) = string_array(env, &["_id", "_display_name", "mime_type", "_size"]) else {
        return Vec::new();
    };
    let selection = mime_types
        .iter()
        .map(|kind| format!("mime_type LIKE '{kind}'"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let Ok(selection) = env.new_string(selection) else {
        return Vec::new();
    };
    let Ok(cursor) = env.call_method(
        &resolver,
        "query",
        "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;[Ljava/lang/String;)Landroid/database/Cursor;",
        &[
            JValue::Object(&uri),
            JValue::Object(&projection),
            JValue::Object(&selection),
            JValue::Object(&JObject::null()),
            JValue::Object(&JObject::null()),
            JValue::Object(&JObject::null()),
        ],
    ) else {
        return Vec::new();
    };
    let Ok(cursor) = cursor.l() else {
        return Vec::new();
    };
    let mut files = Vec::new();
    let mut first = true;
    loop {
        let moved = env
            .call_method(
                &cursor,
                if first { "moveToFirst" } else { "moveToNext" },
                "()Z",
                &[],
            )
            .and_then(|value| value.z())
            .unwrap_or(false);
        first = false;
        if !moved {
            break;
        }
        let Some(id) = text_at(env, &cursor, "_id") else {
            continue;
        };
        let name = text_at(env, &cursor, "_display_name").unwrap_or_default();
        let mime = text_at(env, &cursor, "mime_type").unwrap_or_default();
        let size = env
            .call_method(&cursor, "getLong", "(I)J", &[JValue::Int(1)])
            .and_then(|value| value.j())
            .unwrap_or(0);
        files.push(File {
            name,
            uri: format!("content://media/external/file/{id}"),
            mime,
            size,
        });
    }
    files
}

/// What a file holds.
pub fn read(uri: &str) -> Option<Vec<u8>> {
    let Some(mut guard) = thread() else {
        return None;
    };
    let env = &mut *guard;
    let context = ndk_context::android_context();
    let activity = unsafe { JObject::from_raw(context.context() as jni::sys::jobject) };
    let Ok(resolver) = env
        .call_method(
            &activity,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .and_then(|value| value.l())
    else {
        return None;
    };
    let Ok(text) = env.new_string(uri) else {
        return None;
    };
    let Ok(uri) = env.new_object(
        "android/net/Uri",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&text)],
    ) else {
        return None;
    };
    let Ok(stream) = env
        .call_method(
            &resolver,
            "openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[JValue::Object(&uri)],
        )
        .and_then(|value| value.l())
    else {
        return None;
    };
    let mut bytes = Vec::new();
    loop {
        let Ok(buffer) = env.byte_array_from_slice(&[0_u8; 16 * 1024]) else {
            break;
        };
        let count = env
            .call_method(&stream, "read", "([B)I", &[JValue::Object(&buffer)])
            .and_then(|value| value.i())
            .unwrap_or(0);
        if count <= 0 {
            break;
        }
        let Ok(chunk) = env.convert_byte_array(&buffer) else {
            break;
        };
        let take = chunk.len().min(count as usize);
        bytes.extend_from_slice(&chunk[..take]);
    }
    Some(bytes)
}

/// A file the device holds, as the program sees it.
pub fn chosen(file: &File) -> super::Chosen {
    super::Chosen {
        name: file.name.clone(),
        path: None::<PathBuf>,
        bytes: read(&file.uri),
    }
}

/// The system's own thread, for the JNI.
fn thread() -> Option<jni::AttachGuard<'static>> {
    let context = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) }.ok()?;
    let guard = vm.attach_current_thread().ok()?;
    // The guard is forgotten, but the thread it holds on to is the
    // program's own, and the environment is the thread's.
    let guard: jni::AttachGuard<'static> = unsafe { std::mem::transmute(guard) };
    Some(guard)
}

/// An array of strings, for the system's query.
fn string_array<'local>(
    env: &mut jni::JNIEnv<'local>,
    values: &[&str],
) -> Result<jni::objects::JObject<'local>, jni::errors::Error> {
    let class = env.find_class("java/lang/String")?;
    let array = env.new_object_array(values.len() as i32, &class, jni::objects::JObject::null())?;
    for (index, value) in values.iter().enumerate() {
        let text = env.new_string(*value)?;
        env.set_object_array_element(&array, index as i32, &text)?;
    }
    Ok(array.into())
}

/// The text of a column, if the system has one.
fn text_at(env: &mut jni::JNIEnv<'_>, cursor: &JObject<'_>, column: &str) -> Option<String> {
    let name = env.new_string(column).ok()?;
    let index = env
        .call_method(
            cursor,
            "getColumnIndex",
            "(Ljava/lang/String;)I",
            &[JValue::Object(&name)],
        )
        .and_then(|value| value.i())
        .ok()?;
    if index < 0 {
        return None;
    }
    let text = env
        .call_method(
            cursor,
            "getString",
            "(I)Ljava/lang/String;",
            &[JValue::Int(index)],
        )
        .and_then(|value| value.l())
        .ok()?;
    let text: jni::objects::JString = text.into();
    env.get_string(&text).ok().map(Into::into)
}

use serde::Deserialize;
use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub application: Application,
    pub assets: Assets,
}

#[derive(Debug, Deserialize)]
pub struct Application {
    pub name: String,
    pub binary_name: String,
    pub identifier: String,
    pub description: String,
    pub category: String,
    pub startup_wm_class: String,
    pub macos_bundle_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    pub icon_svg: PathBuf,
}

#[derive(Debug)]
pub struct IconSet {
    pub pngs: Vec<(u32, PathBuf)>,
    pub png_128: PathBuf,
    pub ico: PathBuf,
    pub icns: PathBuf,
}

pub fn load_manifest(path: &Path) -> Result<Manifest> {
    let source = fs::read_to_string(path)?;
    Ok(toml::from_str(&source)?)
}

pub fn resolve_icon_path(manifest_path: &Path, manifest: &Manifest) -> PathBuf {
    manifest_path
        .parent()
        .expect("manifest has parent directory")
        .join(&manifest.assets.icon_svg)
}

pub fn render_icons(svg_path: &Path, output_dir: &Path) -> Result<IconSet> {
    fs::create_dir_all(output_dir)?;
    let svg = fs::read(svg_path)?;
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())?;
    let sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
    let mut pngs = Vec::with_capacity(sizes.len());

    for size in sizes {
        let pixmap = render_icon(&tree, size)?;
        let path = output_dir.join(format!("icon-{size}.png"));
        let file = fs::File::create(&path)?;
        let mut encoder = png::Encoder::new(file, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixmap.data())?;
        pngs.push((size, path));
    }

    let ico = output_dir.join("cringraph.ico");
    write_ico(&ico, &pngs)?;
    let icns = output_dir.join("cringraph.icns");
    write_icns(&icns, &pngs)?;

    Ok(IconSet {
        png_128: output_dir.join("icon-128.png"),
        pngs,
        ico,
        icns,
    })
}

fn render_icon(tree: &resvg::usvg::Tree, size: u32) -> Result<resvg::tiny_skia::Pixmap> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| format!("unable to allocate {size}px icon"))?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

fn write_ico(path: &Path, pngs: &[(u32, PathBuf)]) -> Result<()> {
    let frames = pngs
        .iter()
        .filter(|(size, _)| *size <= 256)
        .collect::<Vec<_>>();
    let header_size = 6 + frames.len() * 16;
    let mut output = Vec::new();
    output.extend_from_slice(&[0, 0, 1, 0]);
    output.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut image_offset = header_size as u32;

    for (size, png_path) in &frames {
        let data = fs::read(png_path)?;
        let dimension = if *size == 256 { 0 } else { *size as u8 };
        output.extend_from_slice(&[dimension, dimension, 0, 0, 1, 0, 32, 0]);
        output.extend_from_slice(&(data.len() as u32).to_le_bytes());
        output.extend_from_slice(&image_offset.to_le_bytes());
        image_offset += data.len() as u32;
    }
    for (_, png_path) in frames {
        output.extend_from_slice(&fs::read(png_path)?);
    }
    fs::write(path, output)?;
    Ok(())
}

fn write_icns(path: &Path, pngs: &[(u32, PathBuf)]) -> Result<()> {
    let types = [
        (16, *b"icp4"),
        (32, *b"icp5"),
        (64, *b"icp6"),
        (128, *b"ic07"),
        (256, *b"ic08"),
        (512, *b"ic09"),
        (1024, *b"ic10"),
    ];
    let mut chunks = Vec::new();
    for (size, code) in types {
        let image = pngs
            .iter()
            .find(|(candidate, _)| *candidate == size)
            .ok_or_else(|| format!("missing {size}px icon"))?;
        let data = fs::read(&image.1)?;
        chunks.extend_from_slice(&code);
        chunks.extend_from_slice(&((data.len() as u32 + 8).to_be_bytes()));
        chunks.extend_from_slice(&data);
    }
    let mut file = fs::File::create(path)?;
    file.write_all(b"icns")?;
    file.write_all(&((chunks.len() as u32 + 8).to_be_bytes()))?;
    file.write_all(&chunks)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        io::BufReader,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn manifest_icon_generates_density_assets_and_native_icon_containers() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let manifest_path = root.join("packaging/app.toml");
        let manifest = load_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.application.identifier, "io.cringraph.native");
        assert!(resolve_icon_path(&manifest_path, &manifest).is_file());

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = env::temp_dir().join(format!("cringraph-icons-{nonce}"));
        let icons = render_icons(&resolve_icon_path(&manifest_path, &manifest), &output).unwrap();

        assert_eq!(icons.pngs.len(), 9);
        let decoder = png::Decoder::new(BufReader::new(fs::File::open(&icons.png_128).unwrap()));
        let reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (128, 128));
        let ico = fs::read(&icons.ico).unwrap();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 7);
        let icns = fs::read(&icons.icns).unwrap();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );

        fs::remove_dir_all(output).unwrap();
    }
}

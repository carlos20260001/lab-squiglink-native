//! The transcendental functions, as the JavaScript has them.
//!
//! Translated from V8's `src/base/ieee754.cc`, version 12.0.267.10, which
//! is itself adapted from fdlibm by Sun Microsystems.
//!
//! # Why the port carries its own
//!
//! The JavaScript this crate is a port of runs on V8, and V8's `Math.cos`,
//! `Math.sin`, `Math.exp` and the rest are V8's own copy of fdlibm rather
//! than the platform's. The two differ: on this machine the platform's
//! `cos` differs from V8's on about one argument in twenty-five, by one
//! unit in the last place, and a unit in the last place is enough to
//! change the last digit of a path.
//!
//! So the port carries its own copy, translated from V8's C++ by
//! `scripts/translate_ieee754.py`, and the two agree to the last bit.
//!
//! # The licence
//!
//! fdlibm's licence, and V8's changes to it, are BSD-style, and they are
//! in `LICENSE.ieee754`.

// The C's own style is kept, for the port is a translation of the C and
// not a rewrite of it: the names are the C's, the brackets are the C's,
// the semicolons are the C's, and the parts the C does not use are the
// C's too.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(redundant_semicolons)]
#![allow(unused_assignments)]
#![allow(unused_imports)]
#![allow(unused_labels)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::unused_unit)]
#![allow(clippy::let_and_return)]
#![allow(clippy::needless_return)]
#![allow(clippy::assign_op_pattern)]
#![allow(clippy::approx_constant)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::type_complexity)]
#![allow(clippy::double_parens)]
#![allow(clippy::eq_op)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::num::Wrapping;

/// The number the JavaScript's own signalling NaN is.
///
/// The C++'s signalling NaN comes out of the function that made it with
/// these bits ✓, and the JavaScript's own has them ✓, and the port's
/// must have them too ✓ — a port that is to be faithful gives the same
/// bits ✓, and a number that is not a number is compared by its bits ✓.
pub const SIGNALING_NAN: f64 = f64::from_bits(0x7ff4_0000_0000_0000);

type Int32 = i32;
type Uint32 = u32;
type Int64 = i64;
type Uint64 = u64;

/// A double, from its bits.
#[inline]
fn from_bits(bits: u64) -> f64 {
    f64::from_bits(bits)
}

/// Take a double apart into its two 32-bit words, the high one first.
#[inline]
fn extract_words(d: f64) -> (u32, u32) {
    let bits = d.to_bits();
    ((bits >> 32) as u32, bits as u32)
}

/// The more significant 32 bits of a double.
#[inline]
fn get_high_word(d: f64) -> u32 {
    (d.to_bits() >> 32) as u32
}

/// The less significant 32 bits of a double.
#[inline]
fn get_low_word(d: f64) -> u32 {
    d.to_bits() as u32
}

/// Put a double together from its two 32-bit words.
#[inline]
fn insert_words(ix0: u32, ix1: u32) -> f64 {
    f64::from_bits(((ix0 as u64) << 32) | (ix1 as u64))
}

/// Set the more significant 32 bits of a double.
#[inline]
fn set_high_word(d: &mut f64, v: u32) {
    let mut bits = d.to_bits();
    bits &= 0x0000_0000_FFFF_FFFF;
    bits |= (v as u64) << 32;
    *d = f64::from_bits(bits);
}

/// The square root, which is the same in every JavaScript and in Rust,
/// for it is the platform's own and it is exact.
#[inline]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// The C++'s `NegateWithWraparound`, which is a negation that does not
/// overflow: the smallest number is its own negation.
#[inline]
fn negate_with_wraparound(a: i32) -> i32 {
    if a == i32::MIN {
        a
    } else {
        -a
    }
}

/// The C++'s `SubWithWraparound`, which is a subtraction that does not
/// overflow.
#[inline]
fn sub_with_wraparound(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

/// Set the less significant 32 bits of a double.
#[inline]
fn set_low_word(d: &mut f64, v: u32) {
    let mut bits = d.to_bits();
    bits &= 0xFFFF_FFFF_0000_0000;
    bits |= v as u64;
    *d = f64::from_bits(bits);
}

const Lg1__1: f64 = 6.666666666666735130e-01;
const Lg2__1: f64 = 3.999999999940941908e-01;
const Lg3__1: f64 = 2.857142874366239149e-01;
const Lg4__1: f64 = 2.222219843214978396e-01;
const Lg5__1: f64 = 1.818357216161805012e-01;
const Lg6__1: f64 = 1.531383769920937332e-01;
const Lg7__1: f64 = 1.479819860511658591e-01;
const zero__1: f64 = 0.0;
const KCOSH_OVERFLOW__1: f64 = 710.4758600739439;
const shuge__1: f64 = 1.0e307;
const half__1: f64 = 0.5;
const two24__1: f64 = 1.67772160000000000000e+07;
const invpio2__1: f64 = 6.36619772367581382433e-01;
const pio2_1__1: f64 = 1.57079632673412561417e+00;
const pio2_1t__1: f64 = 6.07710050650619224932e-11;
const pio2_2__1: f64 = 6.07710050630396597660e-11;
const pio2_2t__1: f64 = 2.02226624879595063154e-21;
const pio2_3__1: f64 = 2.02226624871116645580e-21;
const pio2_3t__1: f64 = 8.47842766036889956997e-32;
const one__1: f64 = 1.0;
const C1__1: f64 = 4.16666666666666019037e-02;
const C2__1: f64 = -1.38888888888741095749e-03;
const C3__1: f64 = 2.48015872894767294178e-05;
const C4__1: f64 = -2.75573143513906633035e-07;
const C5__1: f64 = 2.08757232129817482790e-09;
const C6__1: f64 = -1.13596475577881948265e-11;
const twon24__1: f64 = 5.96046447753906250000e-08;
const S1__1: f64 = -1.66666666666666324348e-01;
const S2__1: f64 = 8.33333333332248946124e-03;
const S3__1: f64 = -1.98412698298579493134e-04;
const S4__1: f64 = 2.75573137070700676789e-06;
const S5__1: f64 = -2.50507602534068634195e-08;
const S6__1: f64 = 1.58969099521155010221e-10;
const pi__1: f64 = 3.1415926535897931160E+00;
const pio2_hi__1: f64 = 1.57079632679489655800e+00;
const pio2_lo__1: f64 = 6.12323399573676603587e-17;
const pS0__1: f64 = 1.66666666666666657415e-01;
const pS1__1: f64 = -3.25565818622400915405e-01;
const pS2__1: f64 = 2.01212532134862925881e-01;
const pS3__1: f64 = -4.00555345006794114027e-02;
const pS4__1: f64 = 7.91534994289814532176e-04;
const pS5__1: f64 = 3.47933107596021167570e-05;
const qS1__1: f64 = -2.40339491173441421878e+00;
const qS2__1: f64 = 2.02094576023350569471e+00;
const qS3__1: f64 = -6.88283971605453293030e-01;
const qS4__1: f64 = 7.70381505559019352791e-02;
const ln2__1: f64 = 6.93147180559945286227e-01;
const huge__1: f64 = 1.0e300;
const pio4_hi__1: f64 = 7.85398163397448278999e-01;
const pi_o_4__1: f64 = 7.8539816339744827900E-01;
const pi_o_2__1: f64 = 1.5707963267948965580E+00;
const o_threshold__1: f64 = 7.09782712893383973096e+02;
const u_threshold__1: f64 = -7.45133219101941108420e+02;
const invln2__1: f64 = 1.44269504088896338700e+00;
const P1__1: f64 = 1.66666666666666019037e-01;
const P2__1: f64 = -2.77777777770155933842e-03;
const P3__1: f64 = 6.61375632143793436117e-05;
const P4__1: f64 = -1.65339022054652515390e-06;
const P5__1: f64 = 4.13813679705723846039e-08;
const E__1: f64 = 2.718281828459045;
const ln2_hi__1: f64 = 6.93147180369123816490e-01;
const ln2_lo__1: f64 = 1.90821492927058770002e-10;
const two54__1: f64 = 1.80143985094819840000e+16;
const Lp1__1: f64 = 6.666666666666735130e-01;
const Lp2__1: f64 = 3.999999999940941908e-01;
const Lp3__1: f64 = 2.857142874366239149e-01;
const Lp4__1: f64 = 2.222219843214978396e-01;
const Lp5__1: f64 = 1.818357216161805012e-01;
const Lp6__1: f64 = 1.531383769920937332e-01;
const Lp7__1: f64 = 1.479819860511658591e-01;
const ivln2hi__1: f64 = 1.44269504072144627571e+00;
const ivln2lo__1: f64 = 1.67517131648865118353e-10;
const ivln10__1: f64 = 4.34294481903251816668e-01;
const log10_2hi__1: f64 = 3.01029995663611771306e-01;
const log10_2lo__1: f64 = 3.69423907715893078616e-13;
const tiny__1: f64 = 1.0e-300;
const Q1__1: f64 = -3.33333333333331316428e-02;
const Q2__1: f64 = 1.58730158725481460165e-03;
const Q3__1: f64 = -7.93650757867487942473e-05;
const Q4__1: f64 = 4.00821782732936239552e-06;
const Q5__1: f64 = -2.01099218183624371326e-07;
const B1__1: Int32 = 715094163;
const B2__1: Int32 = 696219795;
const P0__1: f64 = 1.87595182427177009643;
const two__1: f64 = 2.0;
const two53__1: f64 = 9007199254740992.0;
const L1__1: f64 = 5.99999999999994648725e-01;
const L2__1: f64 = 4.28571428578550184252e-01;
const L3__1: f64 = 3.33333329818377432918e-01;
const L4__1: f64 = 2.72728123808534006489e-01;
const L5__1: f64 = 2.30660745775561754067e-01;
const L6__1: f64 = 2.06975017800338417784e-01;
const lg2__1: f64 = 6.93147180559945286227e-01;
const lg2_h__1: f64 = 6.93147182464599609375e-01;
const lg2_l__1: f64 = -1.90465429995776804525e-09;
const ovt__1: f64 = 8.0085662595372944372e-0017;
const cp__1: f64 = 9.61796693925975554329e-01;
const cp_h__1: f64 = 9.61796700954437255859e-01;
const cp_l__1: f64 = -7.02846165095275826516e-09;
const ivln2__1: f64 = 1.44269504088896338700e+00;
const ivln2_h__1: f64 = 1.44269502162933349609e+00;
const ivln2_l__1: f64 = 1.92596299112661746887e-08;
const KSINH_OVERFLOW__1: f64 = 710.4758600739439;
const TWO_M28__1: f64 = 3.725290298461914e-9;
const LOG_MAXD__1: f64 = 709.7822265625;
const two_over_pi__1: [Int32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];
const npio2_hw__1: [Int32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];
const init_jk__1: [Int32; 4] = [2, 3, 4, 6];
const PIo2__1: [f64; 8] = [
    1.570796251296997,
    7.549789415861596e-08,
    5.390302529957765e-15,
    3.282003415807913e-22,
    1.270655753080676e-29,
    1.2293330898111133e-36,
    2.7337005381646456e-44,
    2.1674168387780482e-51,
];
const xxx__1: [f64; 16] = [
    0.3333333333333341,
    0.13333333333320124,
    0.05396825397622605,
    0.021869488294859542,
    0.0088632398235993,
    0.0035920791075913124,
    0.0014562094543252903,
    0.0005880412408202641,
    0.0002464631348184699,
    7.817944429395571e-05,
    7.140724913826082e-05,
    -1.8558637485527546e-05,
    2.590730518636337e-05,
    1.0,
    0.7853981633974483,
    3.061616997868383e-17,
];
const atanhi__1: [f64; 4] = [
    0.4636476090008061,
    0.7853981633974483,
    0.982793723247329,
    1.5707963267948966,
];
const atanlo__1: [f64; 4] = [
    2.2698777452961687e-17,
    3.061616997868383e-17,
    1.3903311031230998e-17,
    6.123233995736766e-17,
];
const aT__1: [f64; 11] = [
    0.3333333333333293,
    -0.19999999999876483,
    0.14285714272503466,
    -0.11111110405462356,
    0.09090887133436507,
    -0.0769187620504483,
    0.06661073137387531,
    -0.058335701337905735,
    0.049768779946159324,
    -0.036531572744216916,
    0.016285820115365782,
];
const bp__1: [f64; 2] = [1.0, 1.5];
const dp_h__1: [f64; 2] = [0.0, 0.5849624872207642];
const dp_l__1: [f64; 2] = [0.0, 1.350039202129749e-08];
const one_KERNEL_TAN_1: f64 = 1.0;
const pio4: f64 = 0.7853981633974483;
const pio4lo: f64 = 3.061616997868383e-17;
const T: [f64; 16] = [
    0.3333333333333341,
    0.13333333333320124,
    0.05396825397622605,
    0.021869488294859542,
    0.0088632398235993,
    0.0035920791075913124,
    0.0014562094543252903,
    0.0005880412408202641,
    0.0002464631348184699,
    7.817944429395571e-05,
    7.140724913826082e-05,
    -1.8558637485527546e-05,
    2.590730518636337e-05,
    1.0,
    0.7853981633974483,
    3.061616997868383e-17,
];
const two_over_pi_IEEE754_REM_PIO2_1: [Int32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];
const npio2_hw_IEEE754_REM_PIO2_1: [Int32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];
const zero_IEEE754_REM_PIO2_1: f64 = 0.00000000000000000000e+00;
const half_IEEE754_REM_PIO2_1: f64 = 5.00000000000000000000e-01;
const two24_IEEE754_REM_PIO2_1: f64 = 1.67772160000000000000e+07;
const invpio2_IEEE754_REM_PIO2_1: f64 = 6.36619772367581382433e-01;
const pio2_1_IEEE754_REM_PIO2_1: f64 = 1.57079632673412561417e+00;
const pio2_1t_IEEE754_REM_PIO2_1: f64 = 6.07710050650619224932e-11;
const pio2_2_IEEE754_REM_PIO2_1: f64 = 6.07710050630396597660e-11;
const pio2_2t_IEEE754_REM_PIO2_1: f64 = 2.02226624879595063154e-21;
const pio2_3_IEEE754_REM_PIO2_1: f64 = 2.02226624871116645580e-21;
const pio2_3t_IEEE754_REM_PIO2_1: f64 = 8.47842766036889956997e-32;
const init_jk_KERNEL_REM_PIO2_1: [Int32; 4] = [2, 3, 4, 6];
const PIo2_KERNEL_REM_PIO2_1: [f64; 8] = [
    1.570796251296997,
    7.549789415861596e-08,
    5.390302529957765e-15,
    3.282003415807913e-22,
    1.270655753080676e-29,
    1.2293330898111133e-36,
    2.7337005381646456e-44,
    2.1674168387780482e-51,
];
const zero_KERNEL_REM_PIO2_1: f64 = 0.0;
const one_KERNEL_REM_PIO2_1: f64 = 1.0;
const two24_KERNEL_REM_PIO2_1: f64 = 1.67772160000000000000e+07;
const twon24_KERNEL_REM_PIO2_1: f64 = 5.96046447753906250000e-08;
const one_KERNEL_COS_1: f64 = 1.00000000000000000000e+00;
const C1_KERNEL_COS_1: f64 = 4.16666666666666019037e-02;
const C2_KERNEL_COS_1: f64 = -1.38888888888741095749e-03;
const C3_KERNEL_COS_1: f64 = 2.48015872894767294178e-05;
const C4_KERNEL_COS_1: f64 = -2.75573143513906633035e-07;
const C5_KERNEL_COS_1: f64 = 2.08757232129817482790e-09;
const C6_KERNEL_COS_1: f64 = -1.13596475577881948265e-11;
const half_KERNEL_SIN_1: f64 = 5.00000000000000000000e-01;
const S1_KERNEL_SIN_1: f64 = -1.66666666666666324348e-01;
const S2_KERNEL_SIN_1: f64 = 8.33333333332248946124e-03;
const S3_KERNEL_SIN_1: f64 = -1.98412698298579493134e-04;
const S4_KERNEL_SIN_1: f64 = 2.75573137070700676789e-06;
const S5_KERNEL_SIN_1: f64 = -2.50507602534068634195e-08;
const S6_KERNEL_SIN_1: f64 = 1.58969099521155010221e-10;
const xxx_KERNEL_TAN_1: [f64; 16] = [
    0.3333333333333341,
    0.13333333333320124,
    0.05396825397622605,
    0.021869488294859542,
    0.0088632398235993,
    0.0035920791075913124,
    0.0014562094543252903,
    0.0005880412408202641,
    0.0002464631348184699,
    7.817944429395571e-05,
    7.140724913826082e-05,
    -1.8558637485527546e-05,
    2.590730518636337e-05,
    1.0,
    0.7853981633974483,
    3.061616997868383e-17,
];
const one_ACOS_1: f64 = 1.00000000000000000000e+00;
const pi_ACOS_1: f64 = 3.14159265358979311600e+00;
const pio2_hi_ACOS_1: f64 = 1.57079632679489655800e+00;
const pio2_lo_ACOS_1: f64 = 6.12323399573676603587e-17;
const pS0_ACOS_1: f64 = 1.66666666666666657415e-01;
const pS1_ACOS_1: f64 = -3.25565818622400915405e-01;
const pS2_ACOS_1: f64 = 2.01212532134862925881e-01;
const pS3_ACOS_1: f64 = -4.00555345006794114027e-02;
const pS4_ACOS_1: f64 = 7.91534994289814532176e-04;
const pS5_ACOS_1: f64 = 3.47933107596021167570e-05;
const qS1_ACOS_1: f64 = -2.40339491173441421878e+00;
const qS2_ACOS_1: f64 = 2.02094576023350569471e+00;
const qS3_ACOS_1: f64 = -6.88283971605453293030e-01;
const qS4_ACOS_1: f64 = 7.70381505559019352791e-02;
const one_ASIN_1: f64 = 1.00000000000000000000e+00;
const huge_ASIN_1: f64 = 1.000e+300;
const pio2_hi_ASIN_1: f64 = 1.57079632679489655800e+00;
const pio2_lo_ASIN_1: f64 = 6.12323399573676603587e-17;
const pio4_hi_ASIN_1: f64 = 7.85398163397448278999e-01;
const pS0_ASIN_1: f64 = 1.66666666666666657415e-01;
const pS1_ASIN_1: f64 = -3.25565818622400915405e-01;
const pS2_ASIN_1: f64 = 2.01212532134862925881e-01;
const pS3_ASIN_1: f64 = -4.00555345006794114027e-02;
const pS4_ASIN_1: f64 = 7.91534994289814532176e-04;
const pS5_ASIN_1: f64 = 3.47933107596021167570e-05;
const qS1_ASIN_1: f64 = -2.40339491173441421878e+00;
const qS2_ASIN_1: f64 = 2.02094576023350569471e+00;
const qS3_ASIN_1: f64 = -6.88283971605453293030e-01;
const qS4_ASIN_1: f64 = 7.70381505559019352791e-02;
const atanhi_ATAN_1: [f64; 4] = [
    0.4636476090008061,
    0.7853981633974483,
    0.982793723247329,
    1.5707963267948966,
];
const atanlo_ATAN_1: [f64; 4] = [
    2.2698777452961687e-17,
    3.061616997868383e-17,
    1.3903311031230998e-17,
    6.123233995736766e-17,
];
const aT_ATAN_1: [f64; 11] = [
    0.3333333333333293,
    -0.19999999999876483,
    0.14285714272503466,
    -0.11111110405462356,
    0.09090887133436507,
    -0.0769187620504483,
    0.06661073137387531,
    -0.058335701337905735,
    0.049768779946159324,
    -0.036531572744216916,
    0.016285820115365782,
];
const one_ATAN_1: f64 = 1.0;
const huge_ATAN_1: f64 = 1.0e300;
const tiny_ATAN2_1: f64 = 1.0e-300;
const zero_ATAN2_1: f64 = 0.0;
const pi_o_4_ATAN2_1: f64 = 7.8539816339744827900E-01;
const pi_o_2_ATAN2_1: f64 = 1.5707963267948965580E+00;
const pi_ATAN2_1: f64 = 3.1415926535897931160E+00;
const pi_lo: f64 = 1.2246467991473531772E-16;
const one_EXP_1: f64 = 1.0;
const halF: [f64; 2] = [0.5, -0.5];
const o_threshold_EXP_1: f64 = 7.09782712893383973096e+02;
const u_threshold_EXP_1: f64 = -7.45133219101941108420e+02;
const ln2HI: [f64; 2] = [0.6931471803691238, -0.6931471803691238];
const ln2LO: [f64; 2] = [1.9082149292705877e-10, -1.9082149292705877e-10];
const invln2_EXP_1: f64 = 1.44269504088896338700e+00;
const P1_EXP_1: f64 = 1.66666666666666019037e-01;
const P2_EXP_1: f64 = -2.77777777770155933842e-03;
const P3_EXP_1: f64 = 6.61375632143793436117e-05;
const P4_EXP_1: f64 = -1.65339022054652515390e-06;
const P5_EXP_1: f64 = 4.13813679705723846039e-08;
const E_EXP_1: f64 = 2.718281828459045;
const huge_EXP_1: f64 = 1.0e+300;
const twom1000: f64 = 9.33263618503218878990e-302;
const two1023: f64 = 8.988465674311579539e307;
const ln2_hi_LOG_1: f64 = 6.93147180369123816490e-01;
const ln2_lo_LOG_1: f64 = 1.90821492927058770002e-10;
const two54_LOG_1: f64 = 1.80143985094819840000e+16;
const Lg1_LOG_1: f64 = 6.666666666666735130e-01;
const Lg2_LOG_1: f64 = 3.999999999940941908e-01;
const Lg3_LOG_1: f64 = 2.857142874366239149e-01;
const Lg4_LOG_1: f64 = 2.222219843214978396e-01;
const Lg5_LOG_1: f64 = 1.818357216161805012e-01;
const Lg6_LOG_1: f64 = 1.531383769920937332e-01;
const Lg7_LOG_1: f64 = 1.479819860511658591e-01;
const zero_LOG_1: f64 = 0.0;
const two54_LOG2_1: f64 = 1.80143985094819840000e+16;
const ivln2hi_LOG2_1: f64 = 1.44269504072144627571e+00;
const ivln2lo_LOG2_1: f64 = 1.67517131648865118353e-10;
const two54_LOG10_1: f64 = 1.80143985094819840000e+16;
const ivln10_LOG10_1: f64 = 4.34294481903251816668e-01;
const log10_2hi_LOG10_1: f64 = 3.01029995663611771306e-01;
const log10_2lo_LOG10_1: f64 = 3.69423907715893078616e-13;
const ln2_hi_LOG1P_1: f64 = 6.93147180369123816490e-01;
const ln2_lo_LOG1P_1: f64 = 1.90821492927058770002e-10;
const two54_LOG1P_1: f64 = 1.80143985094819840000e+16;
const Lp1_LOG1P_1: f64 = 6.666666666666735130e-01;
const Lp2_LOG1P_1: f64 = 3.999999999940941908e-01;
const Lp3_LOG1P_1: f64 = 2.857142874366239149e-01;
const Lp4_LOG1P_1: f64 = 2.222219843214978396e-01;
const Lp5_LOG1P_1: f64 = 1.818357216161805012e-01;
const Lp6_LOG1P_1: f64 = 1.531383769920937332e-01;
const Lp7_LOG1P_1: f64 = 1.479819860511658591e-01;
const zero_LOG1P_1: f64 = 0.0;
const bp_POW_1: [f64; 2] = [1.0, 1.5];
const dp_h_POW_1: [f64; 2] = [0.0, 0.5849624872207642];
const dp_l_POW_1: [f64; 2] = [0.0, 1.350039202129749e-08];
const zero_POW_1: f64 = 0.0;
const one_POW_1: f64 = 1.0;
const two_POW_1: f64 = 2.0;
const two53_POW_1: f64 = 9007199254740992.0;
const huge_POW_1: f64 = 1.0e300;
const tiny_POW_1: f64 = 1.0e-300;
const L1_POW_1: f64 = 5.99999999999994648725e-01;
const L2_POW_1: f64 = 4.28571428578550184252e-01;
const L3_POW_1: f64 = 3.33333329818377432918e-01;
const L4_POW_1: f64 = 2.72728123808534006489e-01;
const L5_POW_1: f64 = 2.30660745775561754067e-01;
const L6_POW_1: f64 = 2.06975017800338417784e-01;
const P1_POW_1: f64 = 1.66666666666666019037e-01;
const P2_POW_1: f64 = -2.77777777770155933842e-03;
const P3_POW_1: f64 = 6.61375632143793436117e-05;
const P4_POW_1: f64 = -1.65339022054652515390e-06;
const P5_POW_1: f64 = 4.13813679705723846039e-08;
const lg2_POW_1: f64 = 6.93147180559945286227e-01;
const lg2_h_POW_1: f64 = 6.93147182464599609375e-01;
const lg2_l_POW_1: f64 = -1.90465429995776804525e-09;
const ovt_POW_1: f64 = 8.0085662595372944372e-0017;
const cp_POW_1: f64 = 9.61796693925975554329e-01;
const cp_h_POW_1: f64 = 9.61796700954437255859e-01;
const cp_l_POW_1: f64 = -7.02846165095275826516e-09;
const ivln2_POW_1: f64 = 1.44269504088896338700e+00;
const ivln2_h_POW_1: f64 = 1.44269502162933349609e+00;
const ivln2_l_POW_1: f64 = 1.92596299112661746887e-08;
const B1_CBRT_1: Int32 = 715094163;
const B2_CBRT_1: Int32 = 696219795;
const P0_CBRT_1: f64 = 1.87595182427177009643;
const P1_CBRT_1: f64 = -1.88497979543377169875;
const P2_CBRT_1: f64 = 1.621429720105354466140;
const P3_CBRT_1: f64 = -0.758397934778766047437;
const P4_CBRT_1: f64 = 0.145996192886612446982;
const one_EXPM1_1: f64 = 1.0;
const tiny_EXPM1_1: f64 = 1.0e-300;
const o_threshold_EXPM1_1: f64 = 7.09782712893383973096e+02;
const ln2_hi_EXPM1_1: f64 = 6.93147180369123816490e-01;
const ln2_lo_EXPM1_1: f64 = 1.90821492927058770002e-10;
const invln2_EXPM1_1: f64 = 1.44269504088896338700e+00;
const Q1_EXPM1_1: f64 = -3.33333333333331316428e-02;
const Q2_EXPM1_1: f64 = 1.58730158725481460165e-03;
const Q3_EXPM1_1: f64 = -7.93650757867487942473e-05;
const Q4_EXPM1_1: f64 = 4.00821782732936239552e-06;
const Q5_EXPM1_1: f64 = -2.01099218183624371326e-07;
const huge_EXPM1_1: f64 = 1.0e+300;
const KSINH_OVERFLOW_SINH_1: f64 = 710.4758600739439;
const TWO_M28_SINH_1: f64 = 3.725290298461914e-9;
const LOG_MAXD_SINH_1: f64 = 709.7822265625;
const shuge_SINH_1: f64 = 1.0e307;
const KCOSH_OVERFLOW_COSH_1: f64 = 710.4758600739439;
const one_COSH_1: f64 = 1.0;
const half_COSH_1: f64 = 0.5;
const huge_COSH_1: f64 = 1.0e+300;
const tiny_TANH_1: f64 = 1.0e-300;
const one_TANH_1: f64 = 1.0;
const two_TANH_1: f64 = 2.0;
const huge_TANH_1: f64 = 1.0e300;
const one_ASINH_1: f64 = 1.00000000000000000000e+00;
const ln2_ASINH_1: f64 = 6.93147180559945286227e-01;
const huge_ASINH_1: f64 = 1.00000000000000000000e+300;
const one_ACOSH_1: f64 = 1.0;
const ln2_ACOSH_1: f64 = 6.93147180559945286227e-01;
const one_ATANH_1: f64 = 1.0;
const huge_ATANH_1: f64 = 1e300;
const zero_ATANH_1: f64 = 0.0;

/// The C's function, as it is here.
#[inline]
fn k_log1p(f: f64) -> f64 {
    let mut hfsq: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut R: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut t1: f64 = 0.0;
    let mut t2: f64 = 0.0;

    s = f / (2.0 + f);
    z = s * s;
    w = z * z;
    t1 = w * (Lg2_LOG_1 + w * (Lg4_LOG_1 + w * Lg6_LOG_1));
    t2 = z * (Lg1_LOG_1 + w * (Lg3_LOG_1 + w * (Lg5_LOG_1 + w * Lg7_LOG_1)));
    R = t2 + t1;
    hfsq = 0.5 * f * f;
    return s * (hfsq + R);
}

/// The C's function, as it is here.
fn ieee754_rem_pio2(x: f64, y: &mut [f64; 3]) -> Int32 {
    let mut z: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut c_fn: f64 = 0.0;
    let mut tx: [f64; 3] = [0.0; 3];
    let mut e0: Int32 = 0;
    let mut i: Int32 = 0;
    let mut j: Int32 = 0;
    let mut nx: Int32 = 0;
    let mut n: Int32 = 0;
    let mut ix: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut low: Int32 = 0;

    z = 0.0;
    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix <= 0x3FE921FB) {
        y[0] = x;
        y[(1) as usize] = 0.0;
        return 0;
    }
    if (ix < 0x4002D97C) {
        if (hx > 0) {
            z = x - pio2_1_IEEE754_REM_PIO2_1;
            if (ix != 0x3FF921FB) {
                y[0] = z - pio2_1t_IEEE754_REM_PIO2_1;
                y[1] = (z - y[0]) - pio2_1t_IEEE754_REM_PIO2_1;
            } else {
                z -= pio2_2_IEEE754_REM_PIO2_1;
                y[0] = z - pio2_2t_IEEE754_REM_PIO2_1;
                y[1] = (z - y[0]) - pio2_2t_IEEE754_REM_PIO2_1;
            }
            return 1;
        } else {
            z = x + pio2_1_IEEE754_REM_PIO2_1;
            if (ix != 0x3FF921FB) {
                y[0] = z + pio2_1t_IEEE754_REM_PIO2_1;
                y[1] = (z - y[0]) + pio2_1t_IEEE754_REM_PIO2_1;
            } else {
                z += pio2_2_IEEE754_REM_PIO2_1;
                y[0] = z + pio2_2t_IEEE754_REM_PIO2_1;
                y[1] = (z - y[0]) + pio2_2t_IEEE754_REM_PIO2_1;
            }
            return -1;
        }
    }
    if (ix <= 0x413921FB) {
        t = f64::abs(x);
        n = ((t * invpio2_IEEE754_REM_PIO2_1 + half_IEEE754_REM_PIO2_1) as Int32) as Int32;
        c_fn = ((n) as f64);
        r = t - c_fn * pio2_1_IEEE754_REM_PIO2_1;
        w = c_fn * pio2_1t_IEEE754_REM_PIO2_1;
        if (n < 32 && ix != npio2_hw_IEEE754_REM_PIO2_1[(n - 1) as usize]) {
            y[0] = r - w;
        } else {
            let mut high: Int32 = 0;
            j = ix >> 20;
            y[0] = r - w;
            high = (get_high_word(y[0]) as Int32) as Int32;
            i = j - (((high >> 20) & 0x7FF) as i32);
            if (i > 16) {
                t = r;
                w = c_fn * pio2_2_IEEE754_REM_PIO2_1;
                r = t - w;
                w = c_fn * pio2_2t_IEEE754_REM_PIO2_1 - ((t - r) - w);
                y[0] = r - w;
                high = (get_high_word(y[0]) as Int32) as Int32;
                i = j - (((high >> 20) & 0x7FF) as i32);
                if (i > 49) {
                    t = r;
                    w = c_fn * pio2_3_IEEE754_REM_PIO2_1;
                    r = t - w;
                    w = c_fn * pio2_3t_IEEE754_REM_PIO2_1 - ((t - r) - w);
                    y[0] = r - w;
                }
            }
        }
        y[1] = (r - y[0]) - w;
        if (hx < 0) {
            y[0] = -y[0];
            y[1] = -y[1];
            return -n;
        } else {
            return n;
        }
    }

    if (ix >= 0x7FF00000) {
        y[0] = x - x;
        y[1] = x - x;
        return 0;
    }

    low = (get_low_word(x) as Int32) as Int32;
    set_low_word(&mut z, (low) as u32);
    e0 = (ix >> 20) - 1046;
    set_high_word(&mut z, (ix - ((((e0) as Int32) << 20) as Int32)) as u32);
    {
        i = 0;
        while (i < 2) {
            tx[(i) as usize] = (((z) as Int32) as f64);
            z = (z - tx[(i) as usize]) * two24_IEEE754_REM_PIO2_1;
            i += 1
        }
    }
    tx[2] = z;
    nx = 3;
    while (tx[(nx - 1) as usize] == zero_IEEE754_REM_PIO2_1) {
        nx -= 1;
    }
    n = (kernel_rem_pio2(&tx, y, e0, nx, 2, &two_over_pi_IEEE754_REM_PIO2_1)) as Int32;
    if (hx < 0) {
        y[0] = -y[0];
        y[1] = -y[1];
        return -n;
    }
    return n;
}

/// The C's function, as it is here.
fn kernel_rem_pio2(
    x: &[f64; 3],
    y: &mut [f64; 3],
    e0: Int32,
    nx: Int32,
    prec: Int32,
    ipio2: &[Int32; 66],
) -> Int32 {
    let mut jz: Int32 = 0;
    let mut jx: Int32 = 0;
    let mut jv: Int32 = 0;
    let mut jp: Int32 = 0;
    let mut jk: Int32 = 0;
    let mut carry: Int32 = 0;
    let mut n: Int32 = 0;
    let mut iq: [Int32; 20] = [0; 20];
    let mut i: Int32 = 0;
    let mut j: Int32 = 0;
    let mut k: Int32 = 0;
    let mut m: Int32 = 0;
    let mut q0: Int32 = 0;
    let mut ih: Int32 = 0;
    let mut z: f64 = 0.0;
    let mut fw: f64 = 0.0;
    let mut f: [f64; 20] = [0.0; 20];
    let mut fq: [f64; 20] = [0.0; 20];
    let mut q: [f64; 20] = [0.0; 20];

    jk = init_jk_KERNEL_REM_PIO2_1[(prec) as usize];
    jp = jk;

    jx = nx - 1;
    jv = (e0 - 3) / 24;
    if (jv < 0) {
        jv = 0;
    }
    q0 = e0 - 24 * (jv + 1);

    j = jv - jx;
    m = jx + jk;
    {
        i = 0;
        while (i <= m) {
            f[(i) as usize] = if (j < 0) {
                zero_KERNEL_REM_PIO2_1
            } else {
                ((ipio2[(j) as usize]) as f64)
            };
            i += 1;
            j += 1
        }
    }

    {
        i = 0;
        while (i <= jk) {
            {
                j = 0;
                fw = 0.0;
                while (j <= jx) {
                    fw += x[(j) as usize] * f[(jx + i - j) as usize];
                    j += 1
                }
            }
            q[(i) as usize] = fw;
            i += 1
        }
    }

    jz = jk;
    'recompute: loop {
        {
            i = 0;
            j = jz;
            z = q[(jz) as usize];
            while (j > 0) {
                fw = (((twon24_KERNEL_REM_PIO2_1 * z) as Int32) as f64);
                iq[(i) as usize] = ((z - two24_KERNEL_REM_PIO2_1 * fw) as Int32) as Int32;
                z = q[(j - 1) as usize] + fw;
                i += 1;
                j -= 1
            }
        }

        z = (z * 2f64.powi(q0 as i32));
        z -= 8.0 * f64::floor(z * 0.125);
        n = ((z) as Int32) as Int32;
        z -= ((n) as f64);
        ih = 0;
        if (q0 > 0) {
            i = (iq[(jz - 1) as usize] >> (24 - q0));
            n += i;
            iq[(jz - 1) as usize] -= i << (24 - q0);
            ih = iq[(jz - 1) as usize] >> (23 - q0);
        } else if (q0 == 0) {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if (z >= 0.5) {
            ih = 2;
        }

        if (ih > 0) {
            n += 1;
            carry = 0;
            {
                i = 0;
                while (i < jz) {
                    j = iq[(i) as usize];
                    if (carry == 0) {
                        if (j != 0) {
                            carry = 1;
                            iq[(i) as usize] = (0x1000000 - j) as Int32;
                        }
                    } else {
                        iq[(i) as usize] = (0xFFFFFF - j) as Int32;
                    }
                    i += 1
                }
            }
            if (q0 > 0) {
                match q0 {
                    1 => {
                        iq[(jz - 1) as usize] &= 0x7FFFFF;
                    }
                    2 => {
                        iq[(jz - 1) as usize] &= 0x3FFFFF;
                    }
                    _ => {}
                };
            }
            if (ih == 2) {
                z = one_KERNEL_REM_PIO2_1 - z;
                if (carry != 0) {
                    z -= (one_KERNEL_REM_PIO2_1 * 2f64.powi(q0 as i32));
                }
            }
        }

        if (z == zero_KERNEL_REM_PIO2_1) {
            j = 0;
            {
                i = jz - 1;
                while (i >= jk) {
                    j |= iq[(i) as usize];
                    i -= 1
                }
            }
            if (j == 0) {
                {
                    k = 1;
                    while (jk >= k && iq[(jk - k) as usize] == 0) {
                        k += 1
                    }
                }

                {
                    i = jz + 1;
                    while (i <= jz + k) {
                        f[(jx + i) as usize] = (ipio2[(jv + i) as usize]) as f64;
                        {
                            j = 0;
                            fw = 0.0;
                            while (j <= jx) {
                                fw += x[(j) as usize] * f[(jx + i - j) as usize];
                                j += 1
                            }
                        }
                        q[(i) as usize] = fw;
                        i += 1
                    }
                }
                jz += k;
                continue;
            }
        }

        if (z == 0.0) {
            jz -= 1;
            q0 -= 24;
            while (iq[(jz) as usize] == 0) {
                jz -= 1;
                q0 -= 24;
            }
        } else {
            z = (z * 2f64.powi(-q0 as i32));
            if (z >= two24_KERNEL_REM_PIO2_1) {
                fw = (((twon24_KERNEL_REM_PIO2_1 * z) as Int32) as f64);
                iq[(jz) as usize] = (z - two24_KERNEL_REM_PIO2_1 * fw) as Int32;
                jz += 1;
                q0 += 24;
                iq[(jz) as usize] = (fw) as Int32;
            } else {
                iq[(jz) as usize] = (z) as Int32;
            }
        }

        fw = (one_KERNEL_REM_PIO2_1 * 2f64.powi(q0 as i32));
        {
            i = jz;
            while (i >= 0) {
                q[(i) as usize] = fw * (iq[(i) as usize]) as f64;
                fw *= twon24_KERNEL_REM_PIO2_1;
                i -= 1
            }
        }

        {
            i = jz;
            while (i >= 0) {
                {
                    fw = 0.0;
                    k = 0;
                    while (k <= jp && k <= jz - i) {
                        fw += PIo2_KERNEL_REM_PIO2_1[(k) as usize] * q[(i + k) as usize];
                        k += 1
                    }
                }
                fq[(jz - i) as usize] = fw;
                i -= 1
            }
        }

        match prec {
            0 => {
                fw = 0.0;
                {
                    i = jz;
                    while (i >= 0) {
                        fw += fq[(i) as usize];
                        i -= 1
                    }
                }
                y[0] = if (ih == 0) { fw } else { -fw };
            }
            1 | 2 => {
                fw = 0.0;
                {
                    i = jz;
                    while (i >= 0) {
                        fw += fq[(i) as usize];
                        i -= 1
                    }
                }
                y[0] = if (ih == 0) { fw } else { -fw };
                fw = fq[0] - fw;
                {
                    i = 1;
                    while (i <= jz) {
                        fw += fq[(i) as usize];
                        i += 1
                    }
                }
                y[1] = if (ih == 0) { fw } else { -fw };
            }
            3 => {
                {
                    i = jz;
                    while (i > 0) {
                        fw = fq[(i - 1) as usize] + fq[(i) as usize];
                        fq[(i) as usize] += fq[(i - 1) as usize] - fw;
                        fq[(i - 1) as usize] = fw;
                        i -= 1
                    }
                }
                {
                    i = jz;
                    while (i > 1) {
                        fw = fq[(i - 1) as usize] + fq[(i) as usize];
                        fq[(i) as usize] += fq[(i - 1) as usize] - fw;
                        fq[(i - 1) as usize] = fw;
                        i -= 1
                    }
                }
                {
                    fw = 0.0;
                    i = jz;
                    while (i >= 2) {
                        fw += fq[(i) as usize];
                        i -= 1
                    }
                }
                if (ih == 0) {
                    y[0] = fq[0];
                    y[1] = fq[1];
                    y[2] = fw;
                } else {
                    y[0] = -fq[0];
                    y[1] = -fq[1];
                    y[2] = -fw;
                }
            }
            _ => {}
        };
        return n & 7;
    }
}

/// The C's function, as it is here.
#[inline]
fn kernel_cos(x: f64, y: f64) -> f64 {
    let mut a: f64 = 0.0;
    let mut iz: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut qx: f64 = 0.0;
    let mut ix: Int32 = 0;
    ix = (get_high_word(x) as Int32) as Int32;
    ix &= 0x7FFFFFFF;
    if (ix < 0x3E400000) {
        if (((x) as Int32) == 0) {
            return one_KERNEL_COS_1;
        }
    }
    z = x * x;
    r = z
        * (C1_KERNEL_COS_1
            + z * (C2_KERNEL_COS_1
                + z * (C3_KERNEL_COS_1
                    + z * (C4_KERNEL_COS_1 + z * (C5_KERNEL_COS_1 + z * C6_KERNEL_COS_1)))));
    if (ix < 0x3FD33333) {
        return one_KERNEL_COS_1 - (0.5 * z - (z * r - x * y));
    } else {
        if (ix > 0x3FE90000) {
            qx = 0.28125;
        } else {
            qx = insert_words(
                ((ix as u32).wrapping_sub(0x00200000 as u32)) as u32,
                (0) as u32,
            );
        }
        iz = 0.5 * z - qx;
        a = one_KERNEL_COS_1 - qx;
        return a - (iz - (z * r - x * y));
    }
}

/// The C's function, as it is here.
#[inline]
fn kernel_sin(x: f64, y: f64, iy: Int32) -> f64 {
    let mut z: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut v: f64 = 0.0;
    let mut ix: Int32 = 0;
    ix = (get_high_word(x) as Int32) as Int32;
    ix &= 0x7FFFFFFF;
    if (ix < 0x3E400000) {
        if (((x) as Int32) == 0) {
            return x;
        }
    }
    z = x * x;
    v = z * x;
    r = S2_KERNEL_SIN_1
        + z * (S3_KERNEL_SIN_1
            + z * (S4_KERNEL_SIN_1 + z * (S5_KERNEL_SIN_1 + z * S6_KERNEL_SIN_1)));
    if (iy == 0) {
        return x + v * (S1_KERNEL_SIN_1 + z * r);
    } else {
        return x - ((z * (half_KERNEL_SIN_1 * y - v * r) - y) - v * S1_KERNEL_SIN_1);
    }
}

/// The C's function, as it is here.
fn kernel_tan(mut x: f64, mut y: f64, iy: Int32) -> f64 {
    let mut z: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut v: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut ix: Int32 = 0;
    let mut hx: Int32 = 0;

    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix < 0x3E300000) {
        if (((x) as Int32) == 0) {
            let mut low: Int32 = 0;
            low = (get_low_word(x) as Int32) as Int32;
            if (((((ix as u32) | (low as u32)) as u32) | (iy + 1) as u32) == 0) {
                return one_KERNEL_TAN_1 / f64::abs(x);
            } else {
                if (iy == 1) {
                    return x;
                } else {
                    let mut a: f64 = 0.0;
                    let mut t: f64 = 0.0;

                    z = x + y;
                    w = x + y;
                    set_low_word(&mut z, (0) as u32);
                    v = y - (z - x);
                    t = -one_KERNEL_TAN_1 / w;
                    a = -one_KERNEL_TAN_1 / w;
                    set_low_word(&mut t, (0) as u32);
                    s = one_KERNEL_TAN_1 + t * z;
                    return t + a * (s + t * v);
                }
            }
        }
    }
    if (ix >= 0x3FE59428) {
        if (hx < 0) {
            x = -x;
            y = -y;
        }
        z = pio4 - x;
        w = pio4lo - y;
        x = z + w;
        y = 0.0;
    }
    z = x * x;
    w = z * z;

    r = T[1] + w * (T[3] + w * (T[5] + w * (T[7] + w * (T[9] + w * T[11]))));
    v = z * (T[2] + w * (T[4] + w * (T[6] + w * (T[8] + w * (T[10] + w * T[12])))));
    s = z * x;
    r = y + z * (s * (r + v) + y);
    r += T[0] * s;
    w = x + r;
    if (ix >= 0x3FE59428) {
        v = (iy) as f64;
        return ((1 - ((hx >> 30) & 2)) as f64) * (v - 2.0 * (x - (w * w / (w + v) - r)));
    }
    if (iy == 1) {
        return w;
    } else {
        let mut a: f64 = 0.0;
        let mut t: f64 = 0.0;
        z = w;
        set_low_word(&mut z, (0) as u32);
        v = r - (z - x);
        t = -1.0 / w;
        a = -1.0 / w;
        set_low_word(&mut t, (0) as u32);
        s = 1.0 + t * z;
        return t + a * (s + t * v);
    }
}

/// The JavaScript's `Math.cos`, as V8 has it.
pub fn cos(x: f64) -> f64 {
    let mut y: [f64; 3] = [0.0; 3];
    let mut z: f64 = 0.0;
    let mut n: Int32 = 0;
    let mut ix: Int32 = 0;

    ix = (get_high_word(x) as Int32) as Int32;

    ix &= 0x7FFFFFFF;
    if (ix <= 0x3FE921FB) {
        return kernel_cos(x, z);
    } else if (ix >= 0x7FF00000) {
        return x - x;
    } else {
        n = (ieee754_rem_pio2(x, &mut y)) as Int32;
        match n & 3 {
            0 => {
                return kernel_cos(y[0], y[1]);
            }
            1 => {
                return -kernel_sin(y[0], y[1], 1);
            }
            2 => {
                return -kernel_cos(y[0], y[1]);
            }
            _ => {
                return kernel_sin(y[0], y[1], 1);
            }
        };
    }
}

/// The JavaScript's `Math.sin`, as V8 has it.
pub fn sin(x: f64) -> f64 {
    let mut y: [f64; 3] = [0.0; 3];
    let mut z: f64 = 0.0;
    let mut n: Int32 = 0;
    let mut ix: Int32 = 0;

    ix = (get_high_word(x) as Int32) as Int32;

    ix &= 0x7FFFFFFF;
    if (ix <= 0x3FE921FB) {
        return kernel_sin(x, z, 0);
    } else if (ix >= 0x7FF00000) {
        return x - x;
    } else {
        n = (ieee754_rem_pio2(x, &mut y)) as Int32;
        match n & 3 {
            0 => {
                return kernel_sin(y[0], y[1], 1);
            }
            1 => {
                return kernel_cos(y[0], y[1]);
            }
            2 => {
                return -kernel_sin(y[0], y[1], 1);
            }
            _ => {
                return -kernel_cos(y[0], y[1]);
            }
        };
    }
}

/// The JavaScript's `Math.tan`, as V8 has it.
pub fn tan(x: f64) -> f64 {
    let mut y: [f64; 3] = [0.0; 3];
    let mut z: f64 = 0.0;
    let mut n: Int32 = 0;
    let mut ix: Int32 = 0;

    ix = (get_high_word(x) as Int32) as Int32;

    ix &= 0x7FFFFFFF;
    if (ix <= 0x3FE921FB) {
        return kernel_tan(x, z, 1);
    } else if (ix >= 0x7FF00000) {
        return x - x;
    } else {
        n = (ieee754_rem_pio2(x, &mut y)) as Int32;

        return kernel_tan(y[0], y[1], 1 - ((n & 1) << 1));
    }
}

/// The JavaScript's `Math.acos`, as V8 has it.
pub fn acos(x: f64) -> f64 {
    let mut z: f64 = 0.0;
    let mut p: f64 = 0.0;
    let mut q: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut c: f64 = 0.0;
    let mut df: f64 = 0.0;
    let mut hx: Int32 = 0;
    let mut ix: Int32 = 0;
    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix >= 0x3FF00000) {
        let mut lx: Int32 = 0;
        lx = (get_low_word(x) as Int32) as Int32;
        if ((((ix.wrapping_sub(0x3FF00000)) as u32) | (lx as u32)) == 0) {
            if (hx > 0) {
                return 0.0;
            } else {
                return pi_ACOS_1 + 2.0 * pio2_lo_ACOS_1;
            }
        }
        return SIGNALING_NAN;
    }
    if (ix < 0x3FE00000) {
        if (ix <= 0x3C600000) {
            return pio2_hi_ACOS_1 + pio2_lo_ACOS_1;
        }
        z = x * x;
        p = z
            * (pS0_ACOS_1
                + z * (pS1_ACOS_1
                    + z * (pS2_ACOS_1 + z * (pS3_ACOS_1 + z * (pS4_ACOS_1 + z * pS5_ACOS_1)))));
        q = one_ACOS_1 + z * (qS1_ACOS_1 + z * (qS2_ACOS_1 + z * (qS3_ACOS_1 + z * qS4_ACOS_1)));
        r = p / q;
        return pio2_hi_ACOS_1 - (x - (pio2_lo_ACOS_1 - x * r));
    } else if (hx < 0) {
        z = (one_ACOS_1 + x) * 0.5;
        p = z
            * (pS0_ACOS_1
                + z * (pS1_ACOS_1
                    + z * (pS2_ACOS_1 + z * (pS3_ACOS_1 + z * (pS4_ACOS_1 + z * pS5_ACOS_1)))));
        q = one_ACOS_1 + z * (qS1_ACOS_1 + z * (qS2_ACOS_1 + z * (qS3_ACOS_1 + z * qS4_ACOS_1)));
        s = f64::sqrt(z);
        r = p / q;
        w = r * s - pio2_lo_ACOS_1;
        return pi_ACOS_1 - 2.0 * (s + w);
    } else {
        z = (one_ACOS_1 - x) * 0.5;
        s = f64::sqrt(z);
        df = s;
        set_low_word(&mut df, (0) as u32);
        c = (z - df * df) / (s + df);
        p = z
            * (pS0_ACOS_1
                + z * (pS1_ACOS_1
                    + z * (pS2_ACOS_1 + z * (pS3_ACOS_1 + z * (pS4_ACOS_1 + z * pS5_ACOS_1)))));
        q = one_ACOS_1 + z * (qS1_ACOS_1 + z * (qS2_ACOS_1 + z * (qS3_ACOS_1 + z * qS4_ACOS_1)));
        r = p / q;
        w = r * s + c;
        return 2.0 * (df + w);
    }
}

/// The JavaScript's `Math.asin`, as V8 has it.
pub fn asin(x: f64) -> f64 {
    let mut t: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut p: f64 = 0.0;
    let mut q: f64 = 0.0;
    let mut c: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut hx: Int32 = 0;
    let mut ix: Int32 = 0;

    t = 0.0;
    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix >= 0x3FF00000) {
        let mut lx: Int32 = 0;
        lx = (get_low_word(x) as Int32) as Int32;
        if ((((ix.wrapping_sub(0x3FF00000)) as u32) | (lx as u32)) == 0) {
            return x * pio2_hi_ASIN_1 + x * pio2_lo_ASIN_1;
        }
        return SIGNALING_NAN;
    } else if (ix < 0x3FE00000) {
        if (ix < 0x3E400000) {
            if (huge_ASIN_1 + x > one_ASIN_1) {
                return x;
            }
        } else {
            t = x * x;
        }
        p = t
            * (pS0_ASIN_1
                + t * (pS1_ASIN_1
                    + t * (pS2_ASIN_1 + t * (pS3_ASIN_1 + t * (pS4_ASIN_1 + t * pS5_ASIN_1)))));
        q = one_ASIN_1 + t * (qS1_ASIN_1 + t * (qS2_ASIN_1 + t * (qS3_ASIN_1 + t * qS4_ASIN_1)));
        w = p / q;
        return x + x * w;
    }

    w = one_ASIN_1 - f64::abs(x);
    t = w * 0.5;
    p = t
        * (pS0_ASIN_1
            + t * (pS1_ASIN_1
                + t * (pS2_ASIN_1 + t * (pS3_ASIN_1 + t * (pS4_ASIN_1 + t * pS5_ASIN_1)))));
    q = one_ASIN_1 + t * (qS1_ASIN_1 + t * (qS2_ASIN_1 + t * (qS3_ASIN_1 + t * qS4_ASIN_1)));
    s = f64::sqrt(t);
    if (ix >= 0x3FEF3333) {
        w = p / q;
        t = pio2_hi_ASIN_1 - (2.0 * (s + s * w) - pio2_lo_ASIN_1);
    } else {
        w = s;
        set_low_word(&mut w, (0) as u32);
        c = (t - w * w) / (s + w);
        r = p / q;
        p = 2.0 * s * r - (pio2_lo_ASIN_1 - 2.0 * c);
        q = pio4_hi_ASIN_1 - 2.0 * w;
        t = pio4_hi_ASIN_1 - (p - q);
    }
    if (hx > 0) {
        return t;
    } else {
        return -t;
    }
}

/// The JavaScript's `Math.atan`, as V8 has it.
pub fn atan(mut x: f64) -> f64 {
    let mut w: f64 = 0.0;
    let mut s1: f64 = 0.0;
    let mut s2: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut ix: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut id: Int32 = 0;

    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix >= 0x44100000) {
        let mut low: Int32 = 0;
        low = (get_low_word(x) as Int32) as Int32;
        if (ix > 0x7FF00000 || (ix == 0x7FF00000 && (low != 0))) {
            return x + x;
        }
        if (hx > 0) {
            return atanhi_ATAN_1[3] + atanlo_ATAN_1[3];
        } else {
            return -atanhi_ATAN_1[3] - atanlo_ATAN_1[3];
        }
    }
    if (ix < 0x3FDC0000) {
        if (ix < 0x3E400000) {
            if (huge_ATAN_1 + x > one_ATAN_1) {
                return x;
            }
        }
        id = -1;
    } else {
        x = f64::abs(x);
        if (ix < 0x3FF30000) {
            if (ix < 0x3FE60000) {
                id = 0;
                x = (2.0 * x - one_ATAN_1) / (2.0 + x);
            } else {
                id = 1;
                x = (x - one_ATAN_1) / (x + one_ATAN_1);
            }
        } else {
            if (ix < 0x40038000) {
                id = 2;
                x = (x - 1.5) / (one_ATAN_1 + 1.5 * x);
            } else {
                id = 3;
                x = -1.0 / x;
            }
        }
    }

    z = x * x;
    w = z * z;

    s1 = z
        * (aT_ATAN_1[0]
            + w * (aT_ATAN_1[2]
                + w * (aT_ATAN_1[4]
                    + w * (aT_ATAN_1[6] + w * (aT_ATAN_1[8] + w * aT_ATAN_1[10])))));
    s2 = w
        * (aT_ATAN_1[1]
            + w * (aT_ATAN_1[3] + w * (aT_ATAN_1[5] + w * (aT_ATAN_1[7] + w * aT_ATAN_1[9]))));
    if (id < 0) {
        return x - x * (s1 + s2);
    } else {
        z = atanhi_ATAN_1[(id) as usize] - ((x * (s1 + s2) - atanlo_ATAN_1[(id) as usize]) - x);
        return if (hx < 0) { -z } else { z };
    }
}

/// The JavaScript's `Math.atan2`, as V8 has it.
pub fn atan2(y: f64, x: f64) -> f64 {
    let mut z: f64 = 0.0;
    let mut k: Int32 = 0;
    let mut m: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut hy: Int32 = 0;
    let mut ix: Int32 = 0;
    let mut iy: Int32 = 0;
    let mut lx: Int32 = 0;
    let mut ly: Int32 = 0;

    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    (hy, ly) = (extract_words(y).0 as Int32, extract_words(y).1 as Int32);
    iy = ((((hy as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if ((((ix as u32)
        | ((((lx as u32) as u32) | (negate_with_wraparound(lx as i32) as u32) as u32) >> 31)
            as u32)
        > 0x7FF00000)
        || (((iy as u32)
            | ((((ly as u32) as u32) | (negate_with_wraparound(ly as i32) as u32) as u32) >> 31)
                as u32)
            > 0x7FF00000))
    {
        return x + y;
    }
    if (((sub_with_wraparound(hx, 0x3FF00000) as u32) | (lx as u32)) == 0) {
        return atan(y);
    }
    m = (((hy >> 31) & 1) | ((hx >> 30) & 2)) as Int32;

    if (((iy as u32) | (ly as u32)) == 0) {
        match m {
            0 | 1 => {
                return y;
            }
            2 => {
                return pi_ATAN2_1 + tiny_ATAN2_1;
            }
            3 => {
                return -pi_ATAN2_1 - tiny_ATAN2_1;
            }
            _ => {}
        };
    }

    if (((ix as u32) | (lx as u32)) == 0) {
        return if (hy < 0) {
            -pi_o_2_ATAN2_1 - tiny_ATAN2_1
        } else {
            pi_o_2_ATAN2_1 + tiny_ATAN2_1
        };
    }

    if (ix == 0x7FF00000) {
        if (iy == 0x7FF00000) {
            match m {
                0 => {
                    return pi_o_4_ATAN2_1 + tiny_ATAN2_1;
                }
                1 => {
                    return -pi_o_4_ATAN2_1 - tiny_ATAN2_1;
                }
                2 => {
                    return 3.0 * pi_o_4_ATAN2_1 + tiny_ATAN2_1;
                }
                3 => {
                    return -3.0 * pi_o_4_ATAN2_1 - tiny_ATAN2_1;
                }
                _ => {}
            };
        } else {
            match m {
                0 => {
                    return zero_ATAN2_1;
                }
                1 => {
                    return -zero_ATAN2_1;
                }
                2 => {
                    return pi_ATAN2_1 + tiny_ATAN2_1;
                }
                3 => {
                    return -pi_ATAN2_1 - tiny_ATAN2_1;
                }
                _ => {}
            };
        }
    }

    if (iy == 0x7FF00000) {
        return if (hy < 0) {
            -pi_o_2_ATAN2_1 - tiny_ATAN2_1
        } else {
            pi_o_2_ATAN2_1 + tiny_ATAN2_1
        };
    }

    k = (iy - ix) >> 20;
    if (k > 60) {
        z = pi_o_2_ATAN2_1 + 0.5 * pi_lo;
        m &= 1;
    } else if (hx < 0 && k < -60) {
        z = 0.0;
    } else {
        z = atan(f64::abs(y / x));
    }
    match m {
        0 => {
            return z;
        }
        1 => {
            return -z;
        }
        2 => {
            return pi_ATAN2_1 - (z - pi_lo);
        }
        _ => {
            return (z - pi_lo) - pi_ATAN2_1;
        }
    };
}

/// The JavaScript's `Math.exp`, as V8 has it.
pub fn exp(mut x: f64) -> f64 {
    let mut y: f64 = 0.0;
    let mut hi: f64 = 0.0;
    let mut lo: f64 = 0.0;
    let mut c: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut twopk: f64 = 0.0;
    let mut k: Int32 = 0;
    let mut xsb: Int32 = 0;
    let mut hx: Int32 = 0;

    hx = (get_high_word(x) as Int32) as Int32;
    xsb = (hx >> 31) & 1;
    hx &= 0x7FFFFFFF;

    if (hx >= 0x40862E42) {
        if (hx >= 0x7FF00000) {
            let mut lx: Int32 = 0;
            lx = (get_low_word(x) as Int32) as Int32;
            if ((((hx & (0xFFFFF as i32)) as u32) | (lx as u32)) != 0) {
                return x + x;
            } else {
                return if (xsb == 0) { x } else { 0.0 };
            }
        }
        if (x > o_threshold_EXP_1) {
            return huge_EXP_1 * huge_EXP_1;
        }
        if (x < u_threshold_EXP_1) {
            return twom1000 * twom1000;
        }
    }

    if (hx > 0x3FD62E42) {
        if (hx < 0x3FF0A2B2) {
            if (x == 1.0) {
                return E_EXP_1;
            }
            hi = x - ln2HI[(xsb) as usize];
            lo = ln2LO[(xsb) as usize];
            k = 1 - xsb - xsb;
        } else {
            k = ((invln2_EXP_1 * x + halF[(xsb) as usize]) as Int32) as Int32;
            t = k as f64;
            hi = x - t * ln2HI[0];
            lo = t * ln2LO[0];
        }
        x = hi - lo;
    } else if (hx < 0x3E300000) {
        if (huge_EXP_1 + x > one_EXP_1) {
            return one_EXP_1 + x;
        }
    } else {
        k = 0;
    }

    t = x * x;
    if (k >= -1021) {
        twopk = insert_words(
            (0x3FF00000 + ((((k) as Int32) << 20) as Int32)) as u32,
            (0) as u32,
        );
    } else {
        twopk = insert_words(
            (0x3FF00000 + (((k + 1000) as Int32) << 20)) as u32,
            (0) as u32,
        );
    }
    c = x - t * (P1_EXP_1 + t * (P2_EXP_1 + t * (P3_EXP_1 + t * (P4_EXP_1 + t * P5_EXP_1))));
    if (k == 0) {
        return one_EXP_1 - ((x * c) / (c - 2.0) - x);
    } else {
        y = one_EXP_1 - ((lo - (x * c) / (2.0 - c)) - hi);
    }
    if (k >= -1021) {
        if (k == 1024) {
            return y * 2.0 * two1023;
        }
        return y * twopk;
    } else {
        return y * twopk * twom1000;
    }
}

/// The JavaScript's `Math.log`, as V8 has it.
pub fn log(mut x: f64) -> f64 {
    let mut hfsq: f64 = 0.0;
    let mut f: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut R: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut t1: f64 = 0.0;
    let mut t2: f64 = 0.0;
    let mut dk: f64 = 0.0;
    let mut k: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut i: Int32 = 0;
    let mut j: Int32 = 0;
    let mut lx: Int32 = 0;

    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);

    k = 0;
    if (hx < 0x00100000) {
        if ((((hx & (0x7FFFFFFF as i32)) as u32) | (lx as u32)) == 0) {
            return -f64::INFINITY;
        }
        if (hx < 0) {
            return SIGNALING_NAN;
        }
        k -= 54;
        x *= two54_LOG_1;
        hx = (get_high_word(x) as Int32) as Int32;
    }
    if (hx >= 0x7FF00000) {
        return x + x;
    }
    k = k.wrapping_add((hx >> 20) - 1023);
    hx &= 0x000FFFFF;
    i = ((((hx as u32) + 0x95F64u32) as u32) & (0x100000u32 as u32)) as Int32;
    set_high_word(&mut x, (hx | (i ^ 0x3FF00000)) as u32);
    k += (i >> 20);
    f = x - 1.0;
    if ((0x000FFFFF & (2 + hx)) < 3) {
        if (f == zero_LOG_1) {
            if (k == 0) {
                return zero_LOG_1;
            } else {
                dk = ((k) as f64);
                return dk * ln2_hi_LOG_1 + dk * ln2_lo_LOG_1;
            }
        }
        R = f * f * (0.5 - 0.33333333333333333 * f);
        if (k == 0) {
            return f - R;
        } else {
            dk = ((k) as f64);
            return dk * ln2_hi_LOG_1 - ((R - dk * ln2_lo_LOG_1) - f);
        }
    }
    s = f / (2.0 + f);
    dk = ((k) as f64);
    z = s * s;
    i = ((hx as u32).wrapping_sub(0x6147Au32)) as Int32;
    w = z * z;
    j = 0x6B851 - hx;
    t1 = w * (Lg2_LOG_1 + w * (Lg4_LOG_1 + w * Lg6_LOG_1));
    t2 = z * (Lg1_LOG_1 + w * (Lg3_LOG_1 + w * (Lg5_LOG_1 + w * Lg7_LOG_1)));
    i |= j;
    R = t2 + t1;
    if (i > 0) {
        hfsq = 0.5 * f * f;
        if (k == 0) {
            return f - (hfsq - s * (hfsq + R));
        } else {
            return dk * ln2_hi_LOG_1 - ((hfsq - (s * (hfsq + R) + dk * ln2_lo_LOG_1)) - f);
        }
    } else {
        if (k == 0) {
            return f - s * (f - R);
        } else {
            return dk * ln2_hi_LOG_1 - ((s * (f - R) - dk * ln2_lo_LOG_1) - f);
        }
    }
}

/// The JavaScript's `Math.log2`, as V8 has it.
pub fn log2(mut x: f64) -> f64 {
    let mut f: f64 = 0.0;
    let mut hfsq: f64 = 0.0;
    let mut hi: f64 = 0.0;
    let mut lo: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut val_hi: f64 = 0.0;
    let mut val_lo: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut i: Int32 = 0;
    let mut k: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut lx: Int32 = 0;

    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);

    k = 0;
    if (hx < 0x00100000) {
        if ((((hx & (0x7FFFFFFF as i32)) as u32) | (lx as u32)) == 0) {
            return -f64::INFINITY;
        }
        if (hx < 0) {
            return SIGNALING_NAN;
        }
        k -= 54;
        x *= two54_LOG2_1;
        hx = (get_high_word(x) as Int32) as Int32;
    }
    if (hx >= 0x7FF00000) {
        return x + x;
    }
    if (hx == 0x3FF00000 && lx == 0) {
        return 0.0;
    }
    k = k.wrapping_add((hx >> 20) - 1023);
    hx &= 0x000FFFFF;
    i = ((((hx as u32) + 0x95F64u32) as u32) & (0x100000u32 as u32)) as Int32;
    set_high_word(&mut x, (hx | (i ^ 0x3FF00000)) as u32);
    k += (i >> 20);
    y = ((k) as f64);
    f = x - 1.0;
    hfsq = 0.5 * f * f;
    r = k_log1p(f);

    hi = f - hfsq;
    set_low_word(&mut hi, (0) as u32);
    lo = (f - hi) - hfsq + r;
    val_hi = hi * ivln2hi_LOG2_1;
    val_lo = (lo + hi) * ivln2lo_LOG2_1 + lo * ivln2hi_LOG2_1;

    w = y + val_hi;
    val_lo += (y - w) + val_hi;
    val_hi = w;

    return val_lo + val_hi;
}

/// The JavaScript's `Math.log10`, as V8 has it.
pub fn log10(mut x: f64) -> f64 {
    let mut y: f64 = 0.0;
    let mut i: Int32 = 0;
    let mut k: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut lx: Int32 = 0;

    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);

    k = 0;
    if (hx < 0x00100000) {
        if ((((hx & (0x7FFFFFFF as i32)) as u32) | (lx as u32)) == 0) {
            return -f64::INFINITY;
        }
        if (hx < 0) {
            return f64::NAN;
        }
        k -= 54;
        x *= two54_LOG10_1;
        hx = (get_high_word(x) as Int32) as Int32;
        lx = (get_low_word(x) as Int32) as Int32;
    }
    if (hx >= 0x7FF00000) {
        return x + x;
    }
    if (hx == 0x3FF00000 && lx == 0) {
        return 0.0;
    }
    k = k.wrapping_add((hx >> 20) - 1023);

    i = ((((k as u32) as u32) & (0x80000000u32) as u32) >> 31) as Int32;
    hx = (hx & (0x000FFFFF as i32)) | ((0x3FF - i) << 20);
    y = (k + i) as f64;
    set_high_word(&mut x, (hx) as u32);
    set_low_word(&mut x, (lx) as u32);

    let mut z: f64 = y * log10_2lo_LOG10_1 + ivln10_LOG10_1 * log(x);
    return z + y * log10_2hi_LOG10_1;
}

/// The JavaScript's `Math.log1p`, as V8 has it.
pub fn log1p(x: f64) -> f64 {
    let mut hfsq: f64 = 0.0;
    let mut f: f64 = 0.0;
    let mut c: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut R: f64 = 0.0;
    let mut u: f64 = 0.0;
    let mut k: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut hu: Int32 = 0;
    let mut ax: Int32 = 0;

    hx = (get_high_word(x) as Int32) as Int32;
    ax = hx & (0x7FFFFFFF as i32);

    k = 1;
    if (hx < 0x3FDA827A) {
        if (ax >= 0x3FF00000) {
            if (x == -1.0) {
                return -f64::INFINITY;
            } else {
                return SIGNALING_NAN;
            }
        }
        if (ax < 0x3E200000) {
            if (two54_LOG1P_1 + x > zero_LOG1P_1 && ax < 0x3C900000) {
                return x;
            } else {
                return x - x * x * 0.5;
            }
        }
        if (hx > 0 || hx <= ((0xBFD2BEC4u32) as Int32)) {
            k = 0;
            f = x;
            hu = 1;
        }
    }
    if (hx >= 0x7FF00000) {
        return x + x;
    }
    if (k != 0) {
        if (hx < 0x43400000) {
            u = 1.0 + x;
            hu = (get_high_word(u) as Int32) as Int32;
            k = (hu >> 20) - 1023;
            c = if (k > 0) {
                1.0 - (u - x)
            } else {
                x - (u - 1.0)
            };
            c /= u;
        } else {
            u = x;
            hu = (get_high_word(u) as Int32) as Int32;
            k = (hu >> 20) - 1023;
            c = 0.0;
        }
        hu &= 0x000FFFFF;

        if (hu < 0x6A09E) {
            set_high_word(&mut u, (hu | (0x3FF00000 as i32)) as u32);
        } else {
            k += 1;
            set_high_word(&mut u, (hu | (0x3FE00000 as i32)) as u32);
            hu = (0x00100000 - hu) >> 2;
        }
        f = u - 1.0;
    }
    hfsq = 0.5 * f * f;
    if (hu == 0) {
        if (f == zero_LOG1P_1) {
            if (k == 0) {
                return zero_LOG1P_1;
            } else {
                c += (k as f64) * (ln2_lo_LOG1P_1);
                return (k as f64) * (ln2_hi_LOG1P_1) + c;
            }
        }
        R = hfsq * (1.0 - 0.66666666666666666 * f);
        if (k == 0) {
            return f - R;
        } else {
            return (k as f64) * (ln2_hi_LOG1P_1) - ((R - ((k as f64) * (ln2_lo_LOG1P_1) + c)) - f);
        }
    }
    s = f / (2.0 + f);
    z = s * s;
    R = z
        * (Lp1_LOG1P_1
            + z * (Lp2_LOG1P_1
                + z * (Lp3_LOG1P_1
                    + z * (Lp4_LOG1P_1
                        + z * (Lp5_LOG1P_1 + z * (Lp6_LOG1P_1 + z * Lp7_LOG1P_1))))));
    if (k == 0) {
        return f - (hfsq - s * (hfsq + R));
    } else {
        return (k as f64) * (ln2_hi_LOG1P_1)
            - ((hfsq - (s * (hfsq + R) + ((k as f64) * (ln2_lo_LOG1P_1) + c))) - f);
    }
}

/// The JavaScript's `Math.pow`, as V8 has it.
pub fn pow(x: f64, y: f64) -> f64 {
    let mut z: f64 = 0.0;
    let mut ax: f64 = 0.0;
    let mut z_h: f64 = 0.0;
    let mut z_l: f64 = 0.0;
    let mut p_h: f64 = 0.0;
    let mut p_l: f64 = 0.0;
    let mut y1: f64 = 0.0;
    let mut t1: f64 = 0.0;
    let mut t2: f64 = 0.0;
    let mut r: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut u: f64 = 0.0;
    let mut v: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut i: Int32 = 0;
    let mut j: Int32 = 0;
    let mut k: Int32 = 0;
    let mut yisint: Int32 = 0;
    let mut n: Int32 = 0;
    let mut hx: Int32 = 0;
    let mut hy: Int32 = 0;
    let mut ix: Int32 = 0;
    let mut iy: Int32 = 0;
    let mut lx: Int32 = 0;
    let mut ly: Int32 = 0;

    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);
    (hy, ly) = (extract_words(y).0 as Int32, extract_words(y).1 as Int32);
    ix = hx & (0x7fffffff as i32);
    iy = hy & (0x7fffffff as i32);

    if (((iy as u32) | (ly as u32)) == 0) {
        return one_POW_1;
    }

    if (ix > 0x7ff00000
        || ((ix == 0x7ff00000) && (lx != 0))
        || iy > 0x7ff00000
        || ((iy == 0x7ff00000) && (ly != 0)))
    {
        return x + y;
    }

    yisint = 0;
    if (hx < 0) {
        if (iy >= 0x43400000) {
            yisint = 2;
        } else if (iy >= 0x3ff00000) {
            k = (iy >> 20) - 0x3ff;
            if (k > 20) {
                j = ly >> (52 - k);
                if ((j << (52 - k)) == ((ly) as Int32)) {
                    yisint = 2 - (j & 1);
                }
            } else if (ly == 0) {
                j = iy >> (20 - k);
                if ((j << (20 - k)) == iy) {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    if (ly == 0) {
        if (iy == 0x7ff00000) {
            if ((((ix.wrapping_sub(0x3ff00000)) as u32) | (lx as u32)) == 0) {
                return y - y;
            } else if (ix >= 0x3ff00000) {
                return if (hy >= 0) { y } else { zero_POW_1 };
            } else {
                return if (hy < 0) { -y } else { zero_POW_1 };
            }
        }
        if (iy == 0x3ff00000) {
            if (hy < 0) {
                return (one_POW_1) / (x);
            } else {
                return x;
            }
        }
        if (hy == 0x40000000) {
            return x * x;
        }
        if (hy == 0x3fe00000) {
            if (hx >= 0) {
                return f64::sqrt(x);
            }
        }
    }

    ax = f64::abs(x);

    if (lx == 0) {
        if (ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000) {
            z = ax;
            if (hy < 0) {
                z = (one_POW_1) / (z);
            }
            if (hx < 0) {
                if (((ix.wrapping_sub(0x3ff00000)) | yisint) == 0) {
                    z = SIGNALING_NAN;
                } else if (yisint == 1) {
                    z = -z;
                }
            }
            return z;
        }
    }

    n = (hx >> 31) + 1;

    if ((n | yisint) == 0) {
        return SIGNALING_NAN;
    }

    s = one_POW_1;
    if ((n | (yisint - 1)) == 0) {
        s = -one_POW_1;
    }

    if (iy > 0x41e00000) {
        if (iy > 0x43f00000) {
            if (ix <= 0x3fefffff) {
                return if (hy < 0) {
                    huge_POW_1 * huge_POW_1
                } else {
                    tiny_POW_1 * tiny_POW_1
                };
            }
            if (ix >= 0x3ff00000) {
                return if (hy > 0) {
                    huge_POW_1 * huge_POW_1
                } else {
                    tiny_POW_1 * tiny_POW_1
                };
            }
        }

        if (ix < 0x3fefffff) {
            return if (hy < 0) {
                s * huge_POW_1 * huge_POW_1
            } else {
                s * tiny_POW_1 * tiny_POW_1
            };
        }
        if (ix > 0x3ff00000) {
            return if (hy > 0) {
                s * huge_POW_1 * huge_POW_1
            } else {
                s * tiny_POW_1 * tiny_POW_1
            };
        }

        t = ax - one_POW_1;
        w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        u = ivln2_h_POW_1 * t;
        v = t * ivln2_l_POW_1 - w * ivln2_POW_1;
        t1 = u + v;
        set_low_word(&mut t1, (0) as u32);
        t2 = v - (t1 - u);
    } else {
        let mut ss: f64 = 0.0;
        let mut s2: f64 = 0.0;
        let mut s_h: f64 = 0.0;
        let mut s_l: f64 = 0.0;
        let mut t_h: f64 = 0.0;
        let mut t_l: f64 = 0.0;
        n = 0;

        if (ix < 0x00100000) {
            ax *= two53_POW_1;
            n -= 53;
            ix = (get_high_word(ax) as Int32) as Int32;
        }
        n += ((ix) >> 20) - 0x3ff;
        j = ix & (0x000fffff as i32);

        ix = j | (0x3ff00000 as i32);
        if (j <= 0x3988E) {
            k = 0;
        } else if (j < 0xBB67A) {
            k = 1;
        } else {
            k = 0;
            n += 1;
            ix -= 0x00100000;
        }
        set_high_word(&mut ax, (ix) as u32);

        u = ax - bp_POW_1[(k) as usize];
        v = (one_POW_1) / (ax + bp_POW_1[(k) as usize]);
        ss = u * v;
        s_h = ss;
        set_low_word(&mut s_h, (0) as u32);

        t_h = zero_POW_1;
        set_high_word(
            &mut t_h,
            (((ix >> 1) | 0x20000000) + 0x00080000 + (k << 18)) as u32,
        );
        t_l = ax - (t_h - bp_POW_1[(k) as usize]);
        s_l = v * ((u - s_h * t_h) - s_h * t_l);

        s2 = ss * ss;
        r = s2
            * s2
            * (L1_POW_1
                + s2 * (L2_POW_1
                    + s2 * (L3_POW_1 + s2 * (L4_POW_1 + s2 * (L5_POW_1 + s2 * L6_POW_1)))));
        r += s_l * (s_h + ss);
        s2 = s_h * s_h;
        t_h = 3.0 + s2 + r;
        set_low_word(&mut t_h, (0) as u32);
        t_l = r - ((t_h - 3.0) - s2);

        u = s_h * t_h;
        v = s_l * t_h + t_l * ss;

        p_h = u + v;
        set_low_word(&mut p_h, (0) as u32);
        p_l = v - (p_h - u);
        z_h = cp_h_POW_1 * p_h;
        z_l = cp_l_POW_1 * p_h + p_l * cp_POW_1 + dp_l_POW_1[(k) as usize];

        t = ((n) as f64);
        t1 = (((z_h + z_l) + dp_h_POW_1[(k) as usize]) + t);
        set_low_word(&mut t1, (0) as u32);
        t2 = z_l - (((t1 - t) - dp_h_POW_1[(k) as usize]) - z_h);
    }

    y1 = y;
    set_low_word(&mut y1, (0) as u32);
    p_l = (y - y1) * t1 + y * t2;
    p_h = y1 * t1;
    z = p_l + p_h;
    (j, i) = (extract_words(z).0 as Int32, extract_words(z).1 as Int32);
    if (j >= 0x40900000) {
        if (((((j as u32).wrapping_sub(0x40900000 as u32)) as u32) | (i as u32)) != 0) {
            return s * huge_POW_1 * huge_POW_1;
        } else {
            if (p_l + ovt_POW_1 > z - p_h) {
                return s * huge_POW_1 * huge_POW_1;
            }
        }
    } else if ((j & (0x7fffffff as i32)) >= 0x4090cc00) {
        if (((((j as u32).wrapping_sub(0xc090cc00u32)) as u32) | (i as u32)) != 0) {
            return s * tiny_POW_1 * tiny_POW_1;
        } else {
            if (p_l <= z - p_h) {
                return s * tiny_POW_1 * tiny_POW_1;
            }
        }
    }

    i = j & (0x7fffffff as i32);
    k = (i >> 20) - 0x3ff;
    n = 0;
    if (i > 0x3fe00000) {
        n = j + (0x00100000 >> (k + 1));
        k = ((n & (0x7fffffff as i32)) >> 20) - 0x3ff;
        t = zero_POW_1;
        set_high_word(&mut t, (n & !(0x000fffff >> k)) as u32);
        n = ((n & (0x000fffff as i32)) | 0x00100000) >> (20 - k);
        if (j < 0) {
            n = -n;
        }
        p_h -= t;
    }
    t = p_l + p_h;
    set_low_word(&mut t, (0) as u32);
    u = t * lg2_h_POW_1;
    v = (p_l - (t - p_h)) * lg2_POW_1 + t * lg2_l_POW_1;
    z = u + v;
    w = v - (z - u);
    t = z * z;
    t1 = z - t * (P1_POW_1 + t * (P2_POW_1 + t * (P3_POW_1 + t * (P4_POW_1 + t * P5_POW_1))));
    r = (z * t1) / ((t1 - two_POW_1) - (w + z * w));
    z = one_POW_1 - (r - z);
    j = (get_high_word(z) as Int32) as Int32;
    j += ((((n) as Int32) << 20) as Int32);
    if ((j >> 20) <= 0) {
        z = (z * 2f64.powi(n as i32));
    } else {
        let mut tmp: Int32 = 0;
        tmp = (get_high_word(z) as Int32) as Int32;
        set_high_word(&mut z, (tmp + ((((n) as Int32) << 20) as Int32)) as u32);
    }
    return s * z;
}

/// The JavaScript's `Math.cbrt`, as V8 has it.
pub fn cbrt(x: f64) -> f64 {
    let mut hx: Int32 = 0;
    let mut r: f64 = 0.0;
    let mut s: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut sign: Int32 = 0;
    let mut high: Int32 = 0;
    let mut low: Int32 = 0;

    (hx, low) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);
    sign = (((hx as u32) as u32) & (0x80000000u32) as u32) as Int32;
    hx ^= sign;
    if (hx >= 0x7FF00000) {
        return (x + x);
    }

    if (hx < 0x00100000) {
        if (((hx as u32) | (low as u32)) == 0) {
            return (x);
        }
        set_high_word(&mut t, (0x43500000) as u32);
        t *= x;
        high = (get_high_word(t) as Int32) as Int32;
        t = insert_words(
            (sign | ((high & (0x7FFFFFFF as i32)) / 3 + B2_CBRT_1)) as u32,
            (0) as u32,
        );
    } else {
        t = insert_words((sign | (hx / 3 + B1_CBRT_1)) as u32, (0) as u32);
    }

    r = (t * t) * (t / x);
    t = t
        * ((P0_CBRT_1 + r * (P1_CBRT_1 + r * P2_CBRT_1))
            + ((r * r) * r) * (P3_CBRT_1 + r * P4_CBRT_1));

    let mut bits: Uint64 = (t).to_bits();
    bits = (bits + 0x80000000u64) & 0xFFFFFFFFC0000000u64;
    t = f64::from_bits(bits);

    s = t * t;
    r = x / s;
    w = t + t;
    r = (r - t) / (w + r);
    t = t + t * r;

    return (t);
}

/// The JavaScript's `Math.expm1`, as V8 has it.
pub fn expm1(mut x: f64) -> f64 {
    let mut y: f64 = 0.0;
    let mut hi: f64 = 0.0;
    let mut lo: f64 = 0.0;
    let mut c: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut e: f64 = 0.0;
    let mut hxs: f64 = 0.0;
    let mut hfx: f64 = 0.0;
    let mut r1: f64 = 0.0;
    let mut twopk: f64 = 0.0;
    let mut k: Int32 = 0;
    let mut xsb: Int32 = 0;
    let mut hx: Int32 = 0;

    hx = (get_high_word(x) as Int32) as Int32;
    xsb = ((((hx as u32) as u32) & (0x80000000u32) as u32) as Int32) as Int32;
    hx &= 0x7FFFFFFF;

    if (hx >= 0x4043687A) {
        if (hx >= 0x40862E42) {
            if (hx >= 0x7FF00000) {
                let mut low: Int32 = 0;
                low = (get_low_word(x) as Int32) as Int32;
                if ((((hx & (0xFFFFF as i32)) as u32) | (low as u32)) != 0) {
                    return x + x;
                } else {
                    return if (xsb == 0) { x } else { -1.0 };
                }
            }
            if (x > o_threshold_EXPM1_1) {
                return huge_EXPM1_1 * huge_EXPM1_1;
            }
        }
        if (xsb != 0) {
            if (x + tiny_EXPM1_1 < 0.0) {
                return tiny_EXPM1_1 - one_EXPM1_1;
            }
        }
    }

    if (hx > 0x3FD62E42) {
        if (hx < 0x3FF0A2B2) {
            if (xsb == 0) {
                hi = x - ln2_hi_EXPM1_1;
                lo = ln2_lo_EXPM1_1;
                k = 1;
            } else {
                hi = x + ln2_hi_EXPM1_1;
                lo = -ln2_lo_EXPM1_1;
                k = -1;
            }
        } else {
            k = (invln2_EXPM1_1 * x + if (xsb == 0) { 0.5 } else { -0.5 }) as Int32;
            t = k as f64;
            hi = x - t * ln2_hi_EXPM1_1;
            lo = t * ln2_lo_EXPM1_1;
        }
        x = hi - lo;
        c = (hi - x) - lo;
    } else if (hx < 0x3C900000) {
        t = huge_EXPM1_1 + x;
        return x - (t - (huge_EXPM1_1 + x));
    } else {
        k = 0;
    }

    hfx = 0.5 * x;
    hxs = x * hfx;
    r1 = one_EXPM1_1
        + hxs
            * (Q1_EXPM1_1
                + hxs * (Q2_EXPM1_1 + hxs * (Q3_EXPM1_1 + hxs * (Q4_EXPM1_1 + hxs * Q5_EXPM1_1))));
    t = 3.0 - r1 * hfx;
    e = hxs * ((r1 - t) / (6.0 - x * t));
    if (k == 0) {
        return x - (x * e - hxs);
    } else {
        twopk = insert_words(
            (0x3FF00000 + ((((k) as Int32) << 20) as Int32)) as u32,
            (0) as u32,
        );
        e = (x * (e - c) - c);
        e -= hxs;
        if (k == -1) {
            return 0.5 * (x - e) - 0.5;
        }
        if (k == 1) {
            if (x < -0.25) {
                return -2.0 * (e - (x + 0.5));
            } else {
                return one_EXPM1_1 + 2.0 * (x - e);
            }
        }
        if (k <= -2 || k > 56) {
            y = one_EXPM1_1 - (e - x);

            if (k == 1024) {
                y = y * 2.0 * 8.98846567431158e+307;
            } else {
                y = y * twopk;
            }
            return y - one_EXPM1_1;
        }
        t = one_EXPM1_1;
        if (k < 20) {
            set_high_word(&mut t, (0x3FF00000 - (0x200000 >> k)) as u32);
            y = t - (e - x);
            y = y * twopk;
        } else {
            set_high_word(&mut t, ((0x3FF - k) << 20) as u32);
            y = x - (e + t);
            y += one_EXPM1_1;
            y = y * twopk;
        }
    }
    return y;
}

/// The JavaScript's `Math.sinh`, as V8 has it.
pub fn sinh(x: f64) -> f64 {
    let mut h: f64 = if (x < 0.0) { -0.5 } else { 0.5 };

    let mut ax: f64 = f64::abs(x);
    if (ax < 22.0) {
        if (ax < TWO_M28_SINH_1) {
            return x;
        }
        let mut t: f64 = expm1(ax);
        if (ax < 1.0) {
            return h * (2.0 * t - t * t / (t + 1.0));
        }
        return h * (t + t / (t + 1.0));
    }

    if (ax < LOG_MAXD_SINH_1) {
        return h * exp(ax);
    }

    if (ax <= KSINH_OVERFLOW_SINH_1) {
        let mut w: f64 = exp(0.5 * ax);
        let mut t: f64 = h * w;
        return t * w;
    }

    return x * shuge_SINH_1;
}

/// The JavaScript's `Math.cosh`, as V8 has it.
pub fn cosh(x: f64) -> f64 {
    let mut ix: Int32 = 0;

    ix = (get_high_word(x) as Int32) as Int32;
    ix &= 0x7FFFFFFF;

    if (ix < 0x3FD62E43) {
        let mut t: f64 = expm1(f64::abs(x));
        let mut w: f64 = one_COSH_1 + t;

        if (ix < 0x3C800000) {
            return w;
        }
        return one_COSH_1 + (t * t) / (w + w);
    }

    if (ix < 0x40360000) {
        let mut t: f64 = exp(f64::abs(x));
        return half_COSH_1 * t + half_COSH_1 / t;
    }

    if (ix < 0x40862E42) {
        return half_COSH_1 * exp(f64::abs(x));
    }

    if (f64::abs(x) <= KCOSH_OVERFLOW_COSH_1) {
        let mut w: f64 = exp(half_COSH_1 * f64::abs(x));
        let mut t: f64 = half_COSH_1 * w;
        return t * w;
    }

    if (ix >= 0x7FF00000) {
        return x * x;
    }

    return huge_COSH_1 * huge_COSH_1;
}

/// The JavaScript's `Math.tanh`, as V8 has it.
pub fn tanh(x: f64) -> f64 {
    let mut t: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut jx: Int32 = 0;
    let mut ix: Int32 = 0;

    jx = (get_high_word(x) as Int32) as Int32;
    ix = jx & (0x7FFFFFFF as i32);

    if (ix >= 0x7FF00000) {
        if (jx >= 0) {
            return one_TANH_1 / x + one_TANH_1;
        } else {
            return one_TANH_1 / x - one_TANH_1;
        }
    }

    if (ix < 0x40360000) {
        if (ix < 0x3E300000) {
            if (huge_TANH_1 + x > one_TANH_1) {
                return x;
            }
        }
        if (ix >= 0x3FF00000) {
            t = expm1(two_TANH_1 * f64::abs(x));
            z = one_TANH_1 - two_TANH_1 / (t + two_TANH_1);
        } else {
            t = expm1(-two_TANH_1 * f64::abs(x));
            z = -t / (t + two_TANH_1);
        }
    } else {
        z = one_TANH_1 - tiny_TANH_1;
    }
    return if (jx >= 0) { z } else { -z };
}

/// The JavaScript's `Math.asinh`, as V8 has it.
pub fn asinh(x: f64) -> f64 {
    let mut t: f64 = 0.0;
    let mut w: f64 = 0.0;
    let mut hx: Int32 = 0;
    let mut ix: Int32 = 0;
    hx = (get_high_word(x) as Int32) as Int32;
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (ix >= 0x7FF00000) {
        return x + x;
    }
    if (ix < 0x3E300000) {
        if (huge_ASINH_1 + x > one_ASINH_1) {
            return x;
        }
    }
    if (ix > 0x41B00000) {
        w = log(f64::abs(x)) + ln2_ASINH_1;
    } else if (ix > 0x40000000) {
        t = f64::abs(x);
        w = log(2.0 * t + one_ASINH_1 / (f64::sqrt(x * x + one_ASINH_1) + t));
    } else {
        t = x * x;
        w = log1p(f64::abs(x) + t / (one_ASINH_1 + f64::sqrt(one_ASINH_1 + t)));
    }
    if (hx > 0) {
        return w;
    } else {
        return -w;
    }
}

/// The JavaScript's `Math.acosh`, as V8 has it.
pub fn acosh(x: f64) -> f64 {
    let mut t: f64 = 0.0;
    let mut hx: Int32 = 0;
    let mut lx: Int32 = 0;
    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);
    if (hx < 0x3FF00000) {
        return SIGNALING_NAN;
    } else if (hx >= 0x41B00000) {
        if (hx >= 0x7FF00000) {
            return x + x;
        } else {
            return log(x) + ln2_ACOSH_1;
        }
    } else if (((((hx as u32).wrapping_sub(0x3FF00000 as u32)) as u32) | (lx as u32)) == 0) {
        return 0.0;
    } else if (hx > 0x40000000) {
        t = x * x;
        return log(2.0 * x - one_ACOSH_1 / (x + f64::sqrt(t - one_ACOSH_1)));
    } else {
        t = x - one_ACOSH_1;
        return log1p(t + f64::sqrt(2.0 * t + t * t));
    }
}

/// The JavaScript's `Math.atanh`, as V8 has it.
pub fn atanh(mut x: f64) -> f64 {
    let mut t: f64 = 0.0;
    let mut hx: Int32 = 0;
    let mut ix: Int32 = 0;
    let mut lx: Int32 = 0;
    (hx, lx) = (extract_words(x).0 as Int32, extract_words(x).1 as Int32);
    ix = ((((hx as u32) as u32) & (0x7FFFFFFF as u32) as u32) as i32) as Int32;
    if (((ix as u32)
        | ((((lx as u32) as u32) | (negate_with_wraparound(lx as i32) as u32) as u32) >> 31)
            as u32)
        > 0x3FF00000)
    {
        return SIGNALING_NAN;
    }
    if (ix == 0x3FF00000) {
        return if (x > 0.0) {
            f64::INFINITY
        } else {
            -f64::INFINITY
        };
    }
    if (ix < 0x3E300000 && (huge_ATANH_1 + x) > zero_ATANH_1) {
        return x;
    }
    set_high_word(&mut x, (ix) as u32);
    if (ix < 0x3FE00000) {
        t = x + x;
        t = 0.5 * log1p(t + t * x / (one_ATANH_1 - x));
    } else {
        t = 0.5 * log1p((x + x) / (one_ATANH_1 - x));
    }
    if (hx >= 0) {
        return t;
    } else {
        return -t;
    }
}

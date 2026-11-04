//! The read patterns of arrow-array's `from_ffi` before apache/arrow-rs#12010,
//! on a buffer one byte past an aligned boundary, the way a C producer can hand
//! it over.
//!
//! cases:
//!   i32, i64   the last offset of a byte array, read with `*ptr`
//!   slice      variadic buffer lengths, `from_raw_parts::<i64>` then summed
//!   unaligned  the same three reads with `read_unaligned`, as the PR does
//!   atomic     an `AtomicI64` load at base+1 (inside one 16-byte block)
//!   atomic16   an `AtomicI64` load at base+76 (crosses a 16-byte boundary)
//!   cas        an `AtomicI64` compare-exchange at base+1
//! The atomic cases are not in the arrow code path; they show where a CPU can
//! refuse a misaligned access.

use std::hint::black_box;
use std::sync::atomic::{AtomicI64, Ordering};

#[repr(C, align(64))]
struct Block([u8; 128]);

fn main() {
    let case = std::env::args().nth(1).unwrap_or_default();
    let mut block = Box::new(Block([0; 128]));
    let base = block.0.as_mut_ptr();
    let p = unsafe { base.add(1) };
    for (i, v) in [3i64, 7, 11, 13].iter().enumerate() {
        unsafe { (p as *mut i64).add(i).write_unaligned(*v) };
    }
    unsafe { (base.add(76) as *mut i64).write_unaligned(42) };
    let p = black_box(p as *const u8);
    let p12 = black_box(unsafe { base.add(76) } as *const u8);
    println!("arch={} case={case} addr%16={}", std::env::consts::ARCH, p as usize % 16);
    match case.as_str() {
        "i32" => {
            let v = unsafe { *(p as *const i32).add(2) };
            println!("i32 offset = {v} (expected 7)");
        }
        "i64" => {
            let v = unsafe { *(p as *const i64).add(3) };
            println!("i64 offset = {v} (expected 13)");
        }
        "slice" => {
            let n = black_box(4usize);
            let lengths = unsafe { std::slice::from_raw_parts(p as *const i64, n) };
            let sum: i64 = lengths.iter().sum();
            println!("sum of lengths = {sum} (expected 34)");
        }
        "unaligned" => {
            let a = unsafe { (p as *const i32).add(2).read_unaligned() };
            let b = unsafe { (p as *const i64).add(3).read_unaligned() };
            let s: i64 = (0..4).map(|i| unsafe { (p as *const i64).add(i).read_unaligned() }).sum();
            println!("read_unaligned: i32 {a} (7), i64 {b} (13), sum {s} (34)");
        }
        "atomic" => {
            let a = unsafe { &*(p as *const AtomicI64) };
            println!("atomic load = {} (expected 3)", a.load(Ordering::Acquire));
        }
        "atomic16" => {
            let a = unsafe { &*(p12 as *const AtomicI64) };
            println!("atomic load across 16 bytes = {} (expected 42)", a.load(Ordering::Acquire));
        }
        "cas" => {
            let a = unsafe { &*(p as *const AtomicI64) };
            let r = a.compare_exchange(3, 5, Ordering::AcqRel, Ordering::Acquire);
            println!("compare_exchange = {r:?} (expected Ok(3))");
        }
        other => panic!("unknown case {other:?}"),
    }
    drop(block);
}

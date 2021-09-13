//! This is a small example that shows how you can directly do syscalls
//! on x86_64 to Linux from Rust. It also shows you how you can find out
//! how to do this, i.e. what parts of the Linux source codes are
//! relevant to find the relevant information.
//!
//! Linux defines the syscall ABI here:
//! https://github.com/torvalds/linux/blob/master/arch/x86/entry/entry_64.S#L69
//! And here is the table of all supported syscalls:
//! https://github.com/torvalds/linux/blob/master/arch/x86/entry/syscalls/syscall_64.tbl
//! Here you can find the definition of the syscalls:
//! https://github.com/torvalds/linux/blob/master/include/linux/syscalls.h

#![feature(asm)]

use crate::LinuxFileFlags::{O_APPEND, O_CREAT, O_RDONLY, O_WRONLY};
use std::ffi::CStr;
use std::os::raw::c_char;
#[cfg(any(not(target_os = "linux"), not(target_arch = "x86_64")))]
compile_error!("Only works on x86_64 Linux");

/// Small subset of the available Linux syscalls.
#[repr(u64)]
enum LinuxSysCalls {
    Read = 0,
    Write = 1,
    Open = 2,
    WriteV = 20,
}

/// Flags that can be used for the `open()` system call.
/// Flags that can be used here are specified in:
/// - https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/fcntl.h
/// - https://github.com/torvalds/linux/blob/master/include/linux/fcntl.h
///
/// Most of these information are in the manpage: `$ man open`
///
/// Linux defines each variant using the octal number format.
#[repr(u32)]
#[allow(non_camel_case_types, unused)]
enum LinuxFileFlags {
    /// Open for reading only.
    O_RDONLY = 0o0,
    /// Open for writing only.
    O_WRONLY = 0o1,
    /// Opens a file for reading and writing.
    O_RDWR = 0o2,
    /// Create file if it doesn't exist.
    O_CREAT = 0o100,
    /// Append if file has content.
    O_APPEND = 0o2000,
}

/// Wrapper around a Linux syscall with three arguments. It returns
/// the syscall result (or error code) that gets stored in rax.
unsafe fn syscall_3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    asm!(
        "mov rax, {0}",
        "mov rdi, {1}",
        "mov rsi, {2}",
        "mov rdx, {3}",
        "syscall",
        in(reg) num,
        in(reg) arg1,
        in(reg) arg2,
        in(reg) arg3,
    );
    let res;
    asm!(
        "mov {}, rax",
        out(reg) res
    );
    res
}

/// Linux write system call. Works like `write()` in C.
fn sys_write(fd: u64, data: *const u8, len: u64) -> i64 {
    unsafe { syscall_3(LinuxSysCalls::Write as u64, fd, data as u64, len) }
}

/// Opens a file. Works like `open` in C.
fn sys_open(path: *const u8, flags: u32, umode: u16) -> i64 {
    unsafe {
        syscall_3(
            LinuxSysCalls::Open as u64,
            path as u64,
            flags as u64,
            umode as u64,
        )
    }
}

/// Opens a file. Works like `open` in C.
fn sys_read(fd: u64, buf: *mut u8, size: u64) -> i64 {
    unsafe { syscall_3(LinuxSysCalls::Read as u64, fd, buf as u64, size as u64) }
}

/// Small example that prints "hello world" to stdout/the console, by
/// executing a Linux system call directly without libc or another lib.
///
/// After that, it opens/creates "./foo.txt", writes data to it and read
/// the data from it afterwards - everything with manual syscalls.
fn main() {
    // stdout has file descriptor 1 on UNIX
    // Change this to 511 for example and you will get "-9", which
    // is the error code for "bad fd number".
    const STDOUT_FD: u64 = 1;

    let string = b"hello world\n";
    let res = sys_write(STDOUT_FD, string.as_ptr(), string.len() as u64);

    // now use the regular Rust way (println uses a write system call behind the scenes) :)
    print!("bytes written: ");
    if res >= 0 {
        print!("{}", res)
    } else {
        // check error against:
        // - https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/errno-base.h
        // - https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/errno.h
        print!("<error={}>", res);
    }
    println!();

    // -------------------------------------------------------------------
    // now we
    // 1) write to file "foo.txt"
    // 2) read the content from "foo.txt"
    // 3) print the content from "foo.txt" to stdout

    let fd = sys_open(
        // null terminated - important here!
        b"./foo.txt\0".as_ptr(),
        O_CREAT as u32 | O_WRONLY as u32 | O_APPEND as u32,
        0o777,
    );
    if fd < 0 {
        panic!("could not open file: error={}", fd);
    } else {
        // for convenience, I use the rust std lib here (format)
        let msg = format!("opened ./foo.txt with fd={}\n", fd);
        sys_write(STDOUT_FD, msg.as_ptr(), msg.len() as u64);
    }

    // write to the file
    let msg = "hello, this was written to the file\n";
    sys_write(fd as u64, msg.as_ptr(), msg.len() as u64);

    // read from the file; open first for reading
    let fd = sys_open(
        // null terminated - important here!
        b"./foo.txt\0".as_ptr(),
        O_RDONLY as u32,
        0,
    );

    // now do the actual reading
    let mut data = [0_u8; 1024];
    let res = sys_read(fd as u64, data.as_mut_ptr(), data.len() as u64);
    if res >= 0 {
        let msg = format!("read {} bytes from foo.txt\n", res);
        sys_write(STDOUT_FD, msg.as_ptr(), msg.len() as u64);
    } else {
        let msg = format!("error reading the file: {}\n", res);
        sys_write(STDOUT_FD, msg.as_ptr(), msg.len() as u64);
        panic!();
    }
    let res = sys_read(fd as u64, data.as_mut_ptr(), data.len() as u64);
    if res == 0 {
        let msg = "EOF reached :)\n";
        sys_write(STDOUT_FD, msg.as_ptr(), msg.len() as u64);
    } else {
        let msg = "File is longer than the buffer :(\n";
        sys_write(STDOUT_FD, msg.as_ptr(), msg.len() as u64);
    }

    // ------------------------------------------------------------------------
    // Test "hello world" with "writev" system call

    let msgs = [
        // important that all strings are null terminated!
        "Hello \0", "Welt \0", "via writev()\0", "\n\0",
    ]
    // - "s.as_ptr()" -> rust string slice to raw byte pointer
    // - construct null terminated c strings from it
    .map(|s| unsafe { CStr::from_ptr(s.as_ptr() as *const c_char) });
    // println!("{:#?}", msgs);
    // ::<4>: for the stack array with the correct size during compile time
    let res = writev::<4>(STDOUT_FD, &msgs);
    println!("res={}", res);
}

/// Linux write system call. Works like `writev()` in C.
/// Struct iovec is defined here:
/// https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/uio.h#L17
fn sys_writev(fd: u64, iovec: *const u8, vlen: u64) -> i64 {
    unsafe { syscall_3(LinuxSysCalls::WriteV as u64, fd, iovec as u64, vlen) }
}

/// Convenient wrapper around [`sys_writev`]. A high level interface that maps the request
/// into the low-level interface. It takes a list of C-Strings and write all of them at once
/// to the kernel.
fn writev<const N: usize>(fd: u64, msgs: &[&CStr]) -> i64 {
    // in-place definition of the struct
    #[derive(Copy, Clone)]
    #[repr(C)]
    struct iovec {
        iov_base: *const c_char,
        len: u64,
    }
    impl Default for iovec {
        fn default() -> Self {
            Self {
                iov_base: std::ptr::null(),
                len: 0,
            }
        }
    }
    // stack-allocated array
    let mut vector: [iovec; N] = [iovec::default(); N];
    // copy the C-string pointers into the iovec-array
    for (i, cstr) in msgs.iter().enumerate() {
        vector[i].iov_base = cstr.as_ptr();
        vector[i].len = cstr.to_bytes().len() as u64
    }
    // execute the syscall
    sys_writev(fd, vector.as_ptr() as *const u8, msgs.len() as u64)
}

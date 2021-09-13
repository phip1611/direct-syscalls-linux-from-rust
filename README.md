# Direct Syscalls to Linux From Rust On x86_64 (no libc or standard lib)

This is a small and stripped down example that does a `write`
system call to Linux without libc or another library. It prints
"hello world" to the terminal, when you invoke it with `$ cargo run`.
After that, it opens/creates `./foo.txt`, writes data to it and read
the data from it afterwards - everything with manual syscalls.

Apart from `write`, it also implements `writev`, `open`, and `read`.

Furthermore, the file includes the relevant pointers to the Linux
source code on Github where you can find the relevant information.

What I show in this blog post is nothing new or unique, but it is something 
I wish someone would have shown me in this simplicity in my second or third 
semester at university. See the comments in `main.rs` for more details
and links!

*Hint: This needs the nightly version of Rust, because it uses the unstable 
asm feature. Might work on stable in the future.*

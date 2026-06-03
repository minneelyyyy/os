use core::fmt::Write;

use crate::arch;

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            unsafe { arch::outb(0x3f8, c) };
        }

        Ok(())
    }
}

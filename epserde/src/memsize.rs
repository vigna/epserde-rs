use crate::TypeName;

/// Like `core::mem::size_of()` but also for complex objects
pub trait MemSize: TypeName {
    /// The memory size of the structure, in bytes. References are counted
    /// just as pointers.
    fn mem_size(&self) -> usize;

    /// Composite structs should implement this to print their children.
    fn _mem_dbg_recourse_on(
        &self,
        _writer: &mut impl core::fmt::Write,
        _depth: usize,
        _max_depth: usize,
        _type_name: bool,
        _humanize: bool,
    ) -> core::fmt::Result {
        Ok(())
    }

    /// Write the data on `writer` debug infos about the structure memory usage, but expanding only
    /// up to `max_depth` levels of nested structures.
    fn mem_dbg_depth_on(
        &self,
        writer: &mut impl core::fmt::Write,
        depth: usize,
        max_depth: usize,
        field_name: Option<&str>,
        type_name: bool,
        humanize: bool,
    ) -> core::fmt::Result {
        if depth > max_depth {
            return Ok(());
        }
        let indent = "  ".repeat(depth);
        writer.write_str(&indent)?;

        if let Some(field_name) = field_name {
            writer.write_str(field_name)?;
        }

        if field_name.is_some() && type_name {
            writer.write_str(" : ")?;
        }

        if type_name {
            writer.write_str(&Self::type_name())?;
        }

        if field_name.is_some() | type_name {
            writer.write_str(" = ")?;
        }

        if humanize {
            let (value, uom) = crate::utils::humanize_float(self.mem_size() as f64);
            writer.write_fmt(format_args!("{:>7.3}{}", value, uom,))?;
        } else {
            writer.write_fmt(format_args!("{}", self.mem_size()))?;
        }
        writer.write_char('\n')?;

        self._mem_dbg_recourse_on(writer, depth + 1, max_depth, type_name, humanize)
    }

    /// Write to stdout debug infos about the structure memory usage, but expanding only
    /// up to `max_depth` levels of nested structures.
    #[cfg(feature = "std")]
    fn mem_dbg_depth(
        &self,
        depth: usize,
        max_depth: usize,
        type_name: bool,
        humanize: bool,
    ) -> core::fmt::Result {
        struct Wrapper(std::io::Stdout);
        impl core::fmt::Write for Wrapper {
            #[inline(always)]
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                use std::io::Write;
                self.0
                    .lock()
                    .write(s.as_bytes())
                    .map_err(|_| core::fmt::Error)
                    .map(|_| ())
            }
        }
        self.mem_dbg_depth_on(
            &mut Wrapper(std::io::stdout()),
            depth,
            max_depth,
            None,
            type_name,
            humanize,
        )
    }

    /// Write on `writer` debug infos about the structure memory usage, but expanding only
    /// up to `max_depth` levels of nested structures.
    ///
    /// Print debug infos about the structure memory usage, expanding
    /// all levels of nested structures.
    fn mem_dbg_on(&self, writer: &mut impl core::fmt::Write) -> core::fmt::Result {
        self.mem_dbg_depth_on(writer, 0, usize::MAX, None, true, true)
    }

    /// Print debug infos about the structure memory usage, but expanding only
    /// up to `max_depth` levels of nested structures.
    ///
    /// Print debug infos about the structure memory usage, expanding
    /// all levels of nested structures.
    #[cfg(feature = "std")]
    fn mem_dbg(&self) -> core::fmt::Result {
        self.mem_dbg_depth(0, usize::MAX, true, true)
    }
}

macro_rules! impl_memory_size {
    ($($ty:ty),*) => {$(
impl MemSize for $ty {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}
    )*};
}

impl_memory_size! {
    (), bool, char, f32, f64,
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize
}

impl MemSize for &'_ str {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

impl<T: MemSize> MemSize for &'_ [T] {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

impl<T: MemSize> MemSize for Option<T> {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>() - core::mem::size_of::<T>() // this is so we consider the 
            + self.as_ref().map_or(0, |x| x.mem_size())
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
impl<T: MemSize> MemSize for Vec<T> {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>() + self.iter().map(|x| x.mem_size()).sum::<usize>()
    }
}

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
impl<T: MemSize> MemSize for Box<[T]> {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>() + self.iter().map(|x| x.mem_size()).sum::<usize>()
    }
}

#[cfg(feature = "mmap_rs")]
impl MemSize for mmap_rs::Mmap {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

#[cfg(feature = "mmap_rs")]
impl MemSize for mmap_rs::MmapMut {
    #[inline(always)]
    fn mem_size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

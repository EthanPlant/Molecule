use core::marker::PhantomData;

use alloc::fmt;

use super::{addr::VirtAddr, PageSize, PageSize4K};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page<S: PageSize = PageSize4K> {
    start: VirtAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> Page<S> {
    pub fn containing_addr(addr: VirtAddr) -> Self {
        Self {
            start: addr.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    pub fn start_addr(self) -> VirtAddr {
        self.start
    }
}

impl<S: PageSize> fmt::Debug for Page<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "Page[{}]({:#x})",
            S::SIZE_STR,
            usize::from(self.start)
        ))
    }
}

use self::page::Page;
use core::num::NonZeroUsize;
use lru::LruCache;

pub mod page;

type BlockLocation = u64;

/// The maximal number of pages stored in the cache. If more pages are needed,
/// the least recently used pages will be removed.
pub const CACHE_SIZE: usize = 1024;

pub struct Cache {
    pages: LruCache<BlockLocation, Page>,
}

impl Cache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pages: LruCache::new(NonZeroUsize::new(CACHE_SIZE).expect("Page cache size is zero")),
        }
    }

    /// Insert a page into the cache. If the page is already in the cache or if the
    /// cache is full, it return the old page.
    pub fn insert(&mut self, location: BlockLocation, page: Page) -> Option<Page> {
        self.pages.push(location, page).map(|(_, page)| page)
    }

    /// Remove a page from the cache. If the page is not in the cache, return `None`,
    /// otherwise return the removed page.
    pub fn remove(&mut self, location: BlockLocation) -> Option<Page> {
        self.pages.pop(&location)
    }

    /// Get a page from the cache. If the page is not in the cache, return `None`.
    pub fn get(&mut self, location: BlockLocation) -> Option<&mut Page> {
        self.pages.get_mut(&location)
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

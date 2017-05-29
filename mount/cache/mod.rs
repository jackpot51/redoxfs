use std::{cmp, ptr};

use redoxfs::Disk;

use syscall::error::Result;

use self::lru_cache::LruCache;

mod linked_hash_map;
mod lru_cache;

fn copy_memory(src: &[u8], dest: &mut [u8]) -> usize {
    let len = cmp::min(src.len(), dest.len());
    unsafe { ptr::copy(src.as_ptr(), dest.as_mut_ptr(), len) };
    len
}

pub struct Cache<T> {
    inner: T,
    cache: LruCache<u64, [u8; 512]>,
}

impl<T: Disk> Cache<T> {
    pub fn new(inner: T) -> Self {
        Cache {
            inner: inner,
            cache: LruCache::new(65536) // 32 MB cache
        }
    }
}

impl<T: Disk> Disk for Cache<T> {
    fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        // println!("Cache read at {}", block);

        let mut read = 0;
        let mut failed = false;
        for i in 0..(buffer.len() + 511)/512 {
            let block_i = block + i as u64;

            let buffer_i = i * 512;
            let buffer_j = cmp::min(buffer_i + 512, buffer.len());
            let buffer_slice = &mut buffer[buffer_i .. buffer_j];

            if let Some(cache_buf) = self.cache.get_mut(&block_i) {
                read += copy_memory(cache_buf, buffer_slice);
            }else{
                failed = true;
                break;
            }
        }

        if failed {
            self.inner.read_at(block, buffer)?;

            read = 0;
            for i in 0..(buffer.len() + 511)/512 {
                let block_i = block + i as u64;

                let buffer_i = i * 512;
                let buffer_j = cmp::min(buffer_i + 512, buffer.len());
                let buffer_slice = &buffer[buffer_i .. buffer_j];

                let mut cache_buf = [0; 512];
                read += copy_memory(buffer_slice, &mut cache_buf);
                self.cache.insert(block_i, cache_buf);
            }
        }

        Ok(read)
    }

    fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        // println!("Cache write at {}", block);

        self.inner.write_at(block, buffer)?;

        let mut written = 0;
        for i in 0..(buffer.len() + 511)/512 {
            let block_i = block + i as u64;

            let buffer_i = i * 512;
            let buffer_j = cmp::min(buffer_i + 512, buffer.len());
            let buffer_slice = &buffer[buffer_i .. buffer_j];

            let mut cache_buf = [0; 512];
            written += copy_memory(buffer_slice, &mut cache_buf);
            self.cache.insert(block_i, cache_buf);
        }

        Ok(written)
    }

    fn size(&mut self) -> Result<u64> {
        self.inner.size()
    }
}

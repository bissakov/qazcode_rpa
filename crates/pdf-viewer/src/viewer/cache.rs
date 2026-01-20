use image::RgbImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct CacheConfig {
    pub max_gpu_pages: usize,
    pub max_ram_pages: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CacheStats {
    pub gpu_cached_pages: usize,
    pub ram_cached_pages: usize,
    pub gpu_memory_mb: f32,
    pub ram_memory_mb: f32,
    pub total_rendered_pages: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

#[derive(Debug, Clone)]
struct GpuCachedPage {
    data: Arc<RgbImage>,
    access_count: u64,
}

#[derive(Debug, Clone)]
struct RamCachedPage {
    data: Arc<RgbImage>,
    access_count: u64,
}

pub enum CacheLevel {
    Gpu,
    Ram,
    Miss,
}

#[derive(Debug)]
pub struct PageCache {
    config: CacheConfig,
    gpu_cache: HashMap<usize, GpuCachedPage>,
    ram_cache: HashMap<usize, RamCachedPage>,
    stats: CacheStats,
}

impl PageCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            gpu_cache: HashMap::new(),
            ram_cache: HashMap::new(),
            stats: CacheStats {
                gpu_cached_pages: 0,
                ram_cached_pages: 0,
                gpu_memory_mb: 0.0,
                ram_memory_mb: 0.0,
                total_rendered_pages: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
        }
    }

    pub fn get_page(&mut self, page_num: usize) -> Option<Arc<RgbImage>> {
        let key = page_num;

        if let Some(entry) = self.gpu_cache.get_mut(&key) {
            entry.access_count += 1;
            self.stats.cache_hits += 1;
            return Some(entry.data.clone());
        }

        if let Some(entry) = self.ram_cache.get_mut(&key) {
            entry.access_count += 1;
            self.stats.cache_hits += 1;

            let image = entry.data.clone();

            if self.gpu_cache.len() < self.config.max_gpu_pages {
                let gpu_entry = GpuCachedPage {
                    data: image.clone(),
                    access_count: entry.access_count,
                };
                self.gpu_cache.insert(key, gpu_entry);
                self.stats.gpu_cached_pages = self.gpu_cache.len();
                self.update_gpu_memory();
            }

            return Some(image);
        }

        self.stats.cache_misses += 1;
        None
    }

    pub fn put_page(&mut self, page_num: usize, image: Arc<RgbImage>) {
        let key = page_num;
        let image_size_mb = (image.len() as f32 * 3.0) / (1024.0 * 1024.0);

        if self.gpu_cache.len() < self.config.max_gpu_pages {
            let entry = GpuCachedPage {
                data: image.clone(),
                access_count: 1,
            };
            self.gpu_cache.insert(key, entry);
            self.stats.gpu_cached_pages = self.gpu_cache.len();
            self.update_gpu_memory();
        } else if self.ram_cache.len() < self.config.max_ram_pages {
            let entry = RamCachedPage {
                data: image,
                access_count: 1,
            };
            self.ram_cache.insert(key, entry);
            self.stats.ram_cached_pages = self.ram_cache.len();
            self.stats.ram_memory_mb += image_size_mb;
        } else {
            self.evict_lru();

            if self.gpu_cache.len() < self.config.max_gpu_pages {
                let entry = GpuCachedPage {
                    data: image,
                    access_count: 1,
                };
                self.gpu_cache.insert(key, entry);
                self.stats.gpu_cached_pages = self.gpu_cache.len();
                self.update_gpu_memory();
            } else {
                let entry = RamCachedPage {
                    data: image,
                    access_count: 1,
                };
                self.ram_cache.insert(key, entry);
                self.stats.ram_cached_pages = self.ram_cache.len();
                self.stats.ram_memory_mb += image_size_mb;
            }
        }

        self.stats.total_rendered_pages += 1;
    }

    fn evict_lru(&mut self) {
        let mut least_accessed: Option<(usize, u64)> = None;
        let mut is_ram = false;

        for (&key, entry) in &self.ram_cache {
            if let Some((_, min_count)) = least_accessed {
                if entry.access_count < min_count {
                    least_accessed = Some((key, entry.access_count));
                    is_ram = true;
                }
            } else {
                least_accessed = Some((key, entry.access_count));
                is_ram = true;
            }
        }

        for (&key, entry) in &self.gpu_cache {
            if let Some((_, min_count)) = least_accessed {
                if entry.access_count < min_count {
                    least_accessed = Some((key, entry.access_count));
                    is_ram = false;
                }
            } else {
                least_accessed = Some((key, entry.access_count));
                is_ram = false;
            }
        }

        if let Some((key, _)) = least_accessed {
            if is_ram {
                if let Some(entry) = self.ram_cache.remove(&key) {
                    let size_mb = (entry.data.len() as f32 * 3.0) / (1024.0 * 1024.0);
                    self.stats.ram_memory_mb = (self.stats.ram_memory_mb - size_mb).max(0.0);
                    self.stats.ram_cached_pages = self.ram_cache.len();
                }
            } else if self.gpu_cache.remove(&key).is_some() {
                self.stats.gpu_cached_pages = self.gpu_cache.len();
                self.update_gpu_memory();
            }
        }
    }

    fn update_gpu_memory(&mut self) {
        let total_bytes: usize = self
            .gpu_cache
            .values()
            .map(|entry| entry.data.len() * 3)
            .sum();
        self.stats.gpu_memory_mb = (total_bytes as f32) / (1024.0 * 1024.0);
    }

    pub fn clear(&mut self) {
        self.gpu_cache.clear();
        self.ram_cache.clear();
        self.stats.gpu_cached_pages = 0;
        self.stats.ram_cached_pages = 0;
        self.stats.gpu_memory_mb = 0.0;
        self.stats.ram_memory_mb = 0.0;
    }

    pub fn get_stats(&self) -> CacheStats {
        self.stats
    }

    pub fn invalidate_page(&mut self, page_num: usize) {
        self.gpu_cache.retain(|&p, _| p != page_num);
        self.ram_cache.retain(|&p, _| p != page_num);
        self.stats.gpu_cached_pages = self.gpu_cache.len();
        self.stats.ram_cached_pages = self.ram_cache.len();
        self.update_gpu_memory();
    }

    pub fn set_config(&mut self, config: CacheConfig) {
        self.config = config;
        self.clear();
    }
}

impl Default for PageCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

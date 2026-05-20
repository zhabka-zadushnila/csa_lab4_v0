use crate::consts::*;

#[derive(Clone, Copy)]
struct CacheLine {
    tag: u32,
    data: [i32; CACHE_LINE_WORDS],
    valid: bool,
    dirty: bool,
}

pub struct MemController {
    ram: Box<[i32]>,
    cache: [CacheLine; CACHE_LINES],
    pub in_stream: Vec<i32>,
    pub out_stream: Vec<i32>,
    wait_cycles: u32,
}

impl MemController {
    pub fn new() -> Self {
        let ram = vec![0i32; RAM_WORDS];
        MemController {
            ram: ram.into_boxed_slice(),
            cache: [CacheLine {
                tag: 0,
                data: [0; CACHE_LINE_WORDS],
                valid: false,
                dirty: false,
            }; CACHE_LINES],
            in_stream: Vec::new(),
            out_stream: Vec::new(),
            wait_cycles: 0,
        }
    }

    fn cache_index(byte_addr: u32) -> usize {
        ((byte_addr >> ADDR_INDEX_SHIFT) & ADDR_INDEX_MASK) as usize
    }

    fn cache_tag(byte_addr: u32) -> u32 {
        byte_addr >> ADDR_TAG_SHIFT
    }

    fn cache_line_base(byte_addr: u32) -> u32 {
        byte_addr & !ADDR_LINE_MASK
    }

    pub fn read(&mut self, addr: u32) -> i32 {
        if addr == IN_PORT {
            if self.in_stream.is_empty() {
                return 0;
            } else {
                return self.in_stream.remove(0);
            }
        }
        if addr == OUT_PORT {
            return 0;
        }

        let index = Self::cache_index(addr);
        let tag = Self::cache_tag(addr);
        let word_offset = ((addr & ADDR_LINE_MASK) >> 2) as usize;

        if self.cache[index].valid && self.cache[index].tag == tag {
            self.wait_cycles = HIT_CYCLES - 1;
            return self.cache[index].data[word_offset];
        }

        self.fill_cache_line(index, tag, addr);
        self.cache[index].data[word_offset]
    }

    pub fn write(&mut self, addr: u32, value: i32) {
        if addr == OUT_PORT {
            self.out_stream.push(value);
            self.wait_cycles = 0;
            return;
        }
        if addr == IN_PORT {
            return;
        }

        let index = Self::cache_index(addr);
        let tag = Self::cache_tag(addr);
        let word_offset = ((addr & ADDR_LINE_MASK) >> 2) as usize;

        if self.cache[index].valid && self.cache[index].tag == tag {
            self.cache[index].data[word_offset] = value;
            self.cache[index].dirty = true;
            self.wait_cycles = HIT_CYCLES - 1;
            return;
        }

        self.fill_cache_line(index, tag, addr);
        self.cache[index].data[word_offset] = value;
        self.cache[index].dirty = true;
    }

    fn fill_cache_line(&mut self, index: usize, tag: u32, addr: u32) {
        if self.cache[index].valid && self.cache[index].dirty {
            let evict_base =
                (self.cache[index].tag << ADDR_TAG_SHIFT) | ((index as u32) << ADDR_INDEX_SHIFT);
            for i in 0..CACHE_LINE_WORDS {
                self.ram[((evict_base >> 2) + i as u32) as usize] = self.cache[index].data[i];
            }
        }

        let line_base = Self::cache_line_base(addr) as usize;
        let mut data = [0; CACHE_LINE_WORDS];
        for (i, item) in data.iter_mut().enumerate() {
            *item = self.ram[(line_base >> 2) + i];
        }

        self.cache[index] = CacheLine {
            tag,
            data,
            valid: true,
            dirty: false,
        };
        self.wait_cycles = MISS_CYCLES - 1;
    }

    pub fn tick(&mut self) -> bool {
        if self.wait_cycles > 0 {
            self.wait_cycles -= 1;
        }
        self.wait_cycles == 0
    }

    pub fn is_ready(&self) -> bool {
        self.wait_cycles == 0
    }

    pub fn load_ram(&mut self, byte_addr: u32, data: &[i32]) {
        let word_index = (byte_addr >> 2) as usize;
        for (i, &val) in data.iter().enumerate() {
            self.ram[word_index + i] = val;
        }
    }
}

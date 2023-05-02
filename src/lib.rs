#![cfg_attr(not(test), no_std)]

use core::{borrow::Borrow, char::MAX};

use gc_headers::{Tracer, HeapResult, Pointer, GarbageCollectingHeap, HeapError};

#[derive(Copy, Clone, Debug)]
pub struct CopyingHeap<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> {
    heaps : [[u64; HEAP_SIZE]; 2],
    num_of_used_blocks: usize, 
    next_block: usize, 
    copy_to: usize, 
    copy_from: usize,
    pointer_buffer: [Pointer; MAX_BLOCKS],
    blocks_buffer : [usize; MAX_BLOCKS],
    alloc_block_buffer : [bool; MAX_BLOCKS],
    block_sizing_buffer: [usize; MAX_BLOCKS],
    // YOUR CODE HERE
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize> GarbageCollectingHeap for
    CopyingHeap<HEAP_SIZE, MAX_BLOCKS>
{
    fn new() -> Self {
        let mut heaps = [[0 as u64; HEAP_SIZE]; 2];
        let mut pointer_buffer = [Pointer::new(0, MAX_BLOCKS); MAX_BLOCKS];
        let mut blocks_buffer = [0; MAX_BLOCKS];
        let mut alloc_block_buffer = [false; MAX_BLOCKS];
        let mut block_sizing_buffer = [0; MAX_BLOCKS];
        Self{heaps, num_of_used_blocks: 0, next_block: 0, pointer_buffer, copy_to: 1, copy_from: 0, blocks_buffer, alloc_block_buffer, block_sizing_buffer}
    }

    fn load(&self, p: Pointer) -> HeapResult<u64> {
        return HeapResult::Ok(self.heaps[self.copy_from][self.blocks_buffer[p.block_num()] + p.offset()]) ;
    }

    fn store(&mut self, p: Pointer, value: u64) -> HeapResult<()> {
        if p.len() <= p.offset() {
            return HeapResult::Err(HeapError::OffsetTooBig)
        } else {
            self.heaps[self.copy_from][self.blocks_buffer[p.block_num()] + p.offset()] = value;
            p.next();
            return HeapResult::Ok(())
        }
    }

    fn malloc<T: Tracer>(&mut self, num_words: usize, tracer: &T) -> HeapResult<Pointer> {
        
        if num_words + self.next_block - 1 >= HEAP_SIZE { //changed this
            self.heaps[self.copy_to] = [0;HEAP_SIZE];
            self.alloc_block_buffer = [false; MAX_BLOCKS];
            let mut tracer_buffer = [false; MAX_BLOCKS];
            tracer.trace(&mut tracer_buffer);
            self.num_of_used_blocks = 0;
            let mut count = 0;
            for (i, value) in tracer_buffer.iter().enumerate() {
                if *value {
                    for j in 0..self.block_sizing_buffer[i] {
                        self.heaps[self.copy_to][count+j] = self.heaps[self.copy_from][j+self.blocks_buffer[i]];
                    }
                    self.blocks_buffer[i] = count;
                    count += self.block_sizing_buffer[i];
                    self.num_of_used_blocks += 1;
                    self.alloc_block_buffer[i] = true;
                    
                }

            }
            self.next_block = count;
            let tmp = self.copy_from;
            self.copy_from = self.copy_to;
            self.copy_to = tmp;
            self.heaps[self.copy_to] = [0;HEAP_SIZE];

            if self.num_of_used_blocks == MAX_BLOCKS {
                return HeapResult::Err(HeapError::OutOfBlocks);
            }
            if num_words + self.next_block > HEAP_SIZE{
                return  HeapResult::Err(HeapError::OutOfMemory);
            }
            else {
                if self.num_of_used_blocks==MAX_BLOCKS{
                    return HeapResult::Err(HeapError::OutOfBlocks);
                }
                let mut num = 0;
                for i in 0..MAX_BLOCKS{
                    if !self.alloc_block_buffer[i]{
                        num = i;
                        self.alloc_block_buffer[i] = true;
                        break;
                    }
                }
                let p = Pointer::new(num, num_words);
                self.blocks_buffer[num] = self.next_block;
                self.block_sizing_buffer[num] = num_words;
                self.num_of_used_blocks +=1;
                self.next_block += num_words;
                
                self.pointer_buffer[num] = p;
                
                return HeapResult::Ok(p);
                
            }

        } else {
            if self.num_of_used_blocks==MAX_BLOCKS{
                return HeapResult::Err(HeapError::OutOfBlocks);
            }
            let mut num = 0;
            for i in 0..MAX_BLOCKS{
                if !self.alloc_block_buffer[i]{
                    num = i;
                    self.alloc_block_buffer[i] = true;
                    break;
                }
            }
            let p = Pointer::new(num, num_words);
            self.blocks_buffer[num] = self.next_block;
            self.block_sizing_buffer[num] = num_words;
            self.num_of_used_blocks +=1;
            self.next_block += num_words;
            
            self.pointer_buffer[num] = p;
            
            return HeapResult::Ok(p);
        }
    }
}

impl<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>
    CopyingHeap<HEAP_SIZE, MAX_BLOCKS>
{
    pub fn is_allocated(&self, block: usize) -> bool {
        return self.alloc_block_buffer[block]
    }   
            
    pub fn num_allocated_blocks(&self) -> usize {
        return self.num_of_used_blocks
    }       
            
    pub fn size_of(&self, block: usize) -> usize {
        return self.block_sizing_buffer[block]
    }       

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn test_many_blocks() {
        let mut allocator = CopyingHeap::<96, 12>::new();
        let mut tracer = TestTracer::default();
        for request in [2, 10, 4, 8, 6, 12, 6, 24, 4, 8, 2, 8] {
            tracer.allocate_next(request, &mut allocator).unwrap();
        }
        assert_eq!(tracer.len(), allocator.num_allocated_blocks());
        assert!(tracer.matches(&allocator));
        assert_eq!(tracer.total_allocated(), 94);

        match tracer.allocate_next(1, &mut allocator) {
            HeapResult::Ok(_) => panic!("Should be an error!"),
            HeapResult::Err(e) => assert_eq!(e, HeapError::OutOfBlocks),
        }

        tracer.test_in_bounds(&mut allocator);

        for _ in 0..(tracer.len() / 2) {
            tracer.deallocate_next_even();
        }
        assert!(tracer.matches(&allocator));
        assert_eq!(tracer.total_allocated(), 24);

        tracer.test_in_bounds(&mut allocator);

        tracer.allocate_next(4, &mut allocator).unwrap();
        assert_eq!(tracer.len(), allocator.num_allocated_blocks());

        tracer.test_in_bounds(&mut allocator);

        tracer.allocate_next(68, &mut allocator).unwrap();
        assert!(tracer.matches(&allocator));
        assert_eq!(tracer.total_allocated(), 96);

        match tracer.allocate_next(1, &mut allocator) {
            HeapResult::Ok(_) => panic!("Should be an error!"),
            HeapResult::Err(e) => assert_eq!(e, HeapError::OutOfMemory),
        }

        tracer.test_in_bounds(&mut allocator);
    }

    #[test]
    fn test_countdown_allocations() {
        const NUM_WORDS: usize = 1024;
        let mut allocator = CopyingHeap::<NUM_WORDS, NUM_WORDS>::new();
        let mut tracer = CountdownTracer::new(362, &mut allocator);
        while tracer.counts > 0 {
            tracer.iterate(&mut allocator);
        }
    }

    struct CountdownTracer {
        counts: u64,
        count_ptr: Option<Pointer>,
    }

    impl Tracer for CountdownTracer {
        fn trace(&self, blocks_used: &mut [bool]) {
            self.count_ptr.map(|p| {
                blocks_used[p.block_num()] = true;
            });
        }
    }

    impl CountdownTracer {
        fn new<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>(start: u64, allocator: &mut CopyingHeap<HEAP_SIZE, MAX_BLOCKS>) -> Self {
            let mut result = Self {counts: start, count_ptr: None};
            let literal_ptr = allocator.malloc(1, &mut result).unwrap();
            allocator.store(literal_ptr, start).unwrap();
            let stored = allocator.load(literal_ptr).unwrap();
            let count_ptr = allocator.malloc(1, &mut result).unwrap();
            allocator.store(count_ptr, stored).unwrap();
            result.count_ptr = Some(count_ptr);
            result
        }

        fn iterate<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>(&mut self, allocator: &mut CopyingHeap<HEAP_SIZE, MAX_BLOCKS>) {
            let p = allocator.malloc(1, self).unwrap();
            allocator.store(p, 0).unwrap();
            let count = allocator.load(self.count_ptr.unwrap()).unwrap();
            assert_eq!(count, self.counts);
            let zero = allocator.load(p).unwrap();
            assert_eq!(0, zero);
            let p = allocator.malloc(1, self).unwrap();
            allocator.store(p, 18446744073709551615).unwrap();
            let p = allocator.malloc(1, self).unwrap();
            allocator.store(p, 1).unwrap();
            
            println!("looking up {:?}", self.count_ptr.unwrap());
            let count = allocator.load(self.count_ptr.unwrap()).unwrap();
            assert_eq!(count, self.counts);
            let drop = allocator.load(p).unwrap();
            self.counts -= drop;
            let p = allocator.malloc(1, self).unwrap();
            allocator.store(p, self.counts).unwrap();
            self.count_ptr = Some(p);
        }
    }

    #[derive(Default, Debug)]
    struct TestTracer {
        allocations: VecDeque<Pointer>,
    }

    impl Tracer for TestTracer {
        fn trace(&self, blocks_used: &mut [bool]) {
            for p in self.allocations.iter() {
                blocks_used[p.block_num()] = true;
            }
        }
    }

    impl TestTracer {
        fn matches<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>(
            &self,
            allocator: &CopyingHeap<HEAP_SIZE, MAX_BLOCKS>,
        ) -> bool {
            for p in self.allocations.iter() {
                if !allocator.is_allocated(p.block_num()) || allocator.size_of(p.block_num()) != p.len() {
                    return false;
                }
            }
            true
        }

        fn allocate_next<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>(
            &mut self,
            request: usize,
            allocator: &mut CopyingHeap<HEAP_SIZE, MAX_BLOCKS>,
        ) -> HeapResult<()> {
            match allocator.malloc(request, self) {
                HeapResult::Ok(p) => {
                    self.allocations.push_back(p);
                    HeapResult::Ok(())
                }
                HeapResult::Err(e) => HeapResult::Err(e),
            }
        }

        fn deallocate_next_even(&mut self) {
            if self.allocations.len() >= 2 {
                let popped = self.allocations.pop_front().unwrap();
                self.allocations.pop_front().unwrap();
                self.allocations.push_back(popped);
            }
        }

        fn len(&self) -> usize {
            self.allocations.len()
        }

        fn total_allocated(&self) -> usize {
            self.allocations.iter().map(|p| p.len()).sum()
        }

        fn test_in_bounds<const HEAP_SIZE: usize, const MAX_BLOCKS: usize>(
            &self,
            allocator: &mut CopyingHeap<HEAP_SIZE, MAX_BLOCKS>,
        ) {
            let mut value = 0;
            for p in self.allocations.iter() {
                let len = p.len();
                let mut p = Some(*p);
                for _ in 0..len {
                    let pt = p.unwrap();
                    allocator.store(pt, value).unwrap();
                    assert_eq!(value, allocator.load(pt).unwrap());
                    value += 1;
                    p = pt.next();
                }
            }

            value = 0;
            for p in self.allocations.iter() {
                let len = p.len();
                let mut p = Some(*p);
                for _ in 0..len {
                    let pt = p.unwrap();
                    assert_eq!(value, allocator.load(pt).unwrap());
                    value += 1;
                    p = pt.next();
                }
            }
        }
    }
}

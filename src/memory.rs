//! Memory types for dealing with bytes

use crate::{
    common::{Addr, Byte, Word},
    mapper::*,
    serialization::Savable,
    NesResult,
};
use enum_dispatch::enum_dispatch;
use rand::Rng;
use std::{
    fmt,
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

#[enum_dispatch(MapperType)]
pub trait MemRead {
    fn read(&mut self, _addr: Addr) -> Byte {
        0
    }
    fn readw(&mut self, _addr: Word) -> Byte {
        0
    }
    fn peek(&self, _addr: Addr) -> Byte {
        0
    }
    fn peekw(&self, _addr: Word) -> Byte {
        0
    }
}
#[enum_dispatch(MapperType)]
pub trait MemWrite {
    fn write(&mut self, _addr: Addr, _val: Byte) {}
    fn writew(&mut self, _addr: Word, _val: Byte) {}
}
pub trait Bankable {
    type Item;
    fn chunks(&self, size: usize) -> Vec<Self::Item>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

#[derive(Default, Clone)]
pub struct Memory {
    data: Vec<Byte>,
    writable: bool,
}

impl Memory {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let randomize = cfg!(not(feature = "no-randomize-ram"));
        let data = if randomize {
            let mut rng = rand::thread_rng();
            let mut data = Vec::with_capacity(capacity);
            for _ in 0..capacity {
                data.push(rng.gen_range(0x00, 0xFF));
            }
            data
        } else {
            vec![0; capacity]
        };
        Self {
            data,
            writable: true,
        }
    }

    pub fn from_bytes(bytes: &[Byte]) -> Self {
        let mut memory = Self::with_capacity(bytes.len());
        memory.data = bytes.to_vec();
        memory
    }

    pub fn rom(capacity: usize) -> Self {
        let mut rom = Self::with_capacity(capacity);
        rom.writable = false;
        rom
    }
    pub fn rom_from_bytes(bytes: &[Byte]) -> Self {
        let mut rom = Self::rom(bytes.len());
        rom.data = bytes.to_vec();
        rom
    }

    pub fn ram(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
    pub fn ram_from_bytes(bytes: &[Byte]) -> Self {
        let mut ram = Self::ram(bytes.len());
        ram.data = bytes.to_vec();
        ram
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl MemRead for Memory {
    fn read(&mut self, addr: Addr) -> Byte {
        self.peek(addr)
    }
    fn readw(&mut self, addr: Word) -> Byte {
        self.peekw(addr)
    }
    fn peek(&self, addr: Addr) -> Byte {
        self.peekw(addr as Word)
    }
    fn peekw(&self, addr: Word) -> Byte {
        if !self.data.is_empty() {
            let addr = addr % self.data.len();
            self.data[addr]
        } else {
            0
        }
    }
}

impl MemWrite for Memory {
    fn write(&mut self, addr: Addr, val: Byte) {
        self.writew(addr as Word, val);
    }
    fn writew(&mut self, addr: Word, val: Byte) {
        if self.writable && !self.data.is_empty() {
            let addr = addr % self.data.len();
            self.data[addr] = val;
        }
    }
}

impl Bankable for Memory {
    type Item = Self;

    fn chunks(&self, size: usize) -> Vec<Memory> {
        let mut chunks: Vec<Memory> = Vec::new();
        for slice in self.data.chunks(size) {
            let mut chunk = Memory::from_bytes(slice);
            chunk.writable = self.writable;
            chunks.push(chunk);
        }
        chunks
    }
    fn len(&self) -> usize {
        self.len()
    }
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl Savable for Memory {
    fn save<F: Write>(&self, fh: &mut F) -> NesResult<()> {
        self.data.save(fh)?;
        self.writable.save(fh)?;
        Ok(())
    }
    fn load<F: Read>(&mut self, fh: &mut F) -> NesResult<()> {
        self.data.load(fh)?;
        self.writable.load(fh)?;
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct Banks<T>
where
    T: MemRead + MemWrite + Bankable,
{
    banks: Vec<T::Item>,
    size: usize,
}

impl<T> Banks<T>
where
    T: MemRead + MemWrite + Bankable,
{
    pub fn new() -> Self {
        Self {
            banks: vec![],
            size: 0usize,
        }
    }

    pub fn init(data: &T, size: usize) -> Self {
        let mut banks: Vec<T::Item> = Vec::with_capacity(data.len());
        if data.len() > 0 {
            for bank in data.chunks(size) {
                banks.push(bank);
            }
        }
        Self { banks, size }
    }
}

impl<T> Deref for Banks<T>
where
    T: MemRead + MemWrite + Bankable,
{
    type Target = Vec<T::Item>;
    fn deref(&self) -> &Vec<T::Item> {
        &self.banks
    }
}

impl<T> DerefMut for Banks<T>
where
    T: MemRead + MemWrite + Bankable,
{
    fn deref_mut(&mut self) -> &mut Vec<T::Item> {
        &mut self.banks
    }
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        write!(
            f,
            "Memory {{ data: {} KB, writable: {} }}",
            self.data.len() / 1024,
            self.writable
        )
    }
}

impl<T> fmt::Debug for Banks<T>
where
    T: MemRead + MemWrite + Bankable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        write!(
            f,
            "Bank {{ len: {}, size: {} KB  }}",
            self.banks.len(),
            self.size / 1024,
        )
    }
}

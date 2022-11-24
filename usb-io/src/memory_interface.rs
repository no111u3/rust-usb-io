use core::fmt::Debug;
pub trait MemoryInterface {
    type Error;

    fn try_read8(&self, address: u32) -> Result<u8, Self::Error>;
    fn try_read16(&self, address: u32) -> Result<u16, Self::Error>;
    fn try_read32(&self, address: u32) -> Result<u32, Self::Error>;

    fn try_write8(&self, address: u32, value: u8) -> Result<(), Self::Error>;
    fn try_write16(&self, address: u32, value: u16) -> Result<(), Self::Error>;
    fn try_write32(&self, address: u32, value: u32) -> Result<(), Self::Error>;
}

pub trait InfallibleMemoryInterface {
    fn read8(&self, address: u32) -> u8;
    fn read16(&self, address: u32) -> u16;
    fn read32(&self, address: u32) -> u32;

    fn write8(&self, address: u32, value: u8);
    fn write16(&self, address: u32, value: u16);
    fn write32(&self, address: u32, value: u32);
}

impl<E, T> InfallibleMemoryInterface for T
where
    E: Debug,
    T: MemoryInterface<Error = E>,
{
    fn read8(&self, address: u32) -> u8 {
        self.try_read8(address).unwrap()
    }

    fn read16(&self, address: u32) -> u16 {
        self.try_read16(address).unwrap()
    }

    fn read32(&self, address: u32) -> u32 {
        self.try_read32(address).unwrap()
    }

    fn write8(&self, address: u32, value: u8) {
        self.try_write8(address, value).unwrap()
    }

    fn write16(&self, address: u32, value: u16) {
        self.try_write16(address, value).unwrap()
    }

    fn write32(&self, address: u32, value: u32) {
        self.try_write32(address, value).unwrap()
    }
}

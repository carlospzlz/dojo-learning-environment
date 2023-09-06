pub struct DMA {
    control: u32,
}

impl DMA {
    pub fn new() -> Self {
        Self {
            // Reset value taken from Lionel Flandrin
            control: 0x07654321,
        }
    }
    pub fn write_register(&mut self, control: u32) -> () {
        self.control = control;
    }

    pub fn read_register(&self, offset: u32) -> u32 {
        self.control
    }
}

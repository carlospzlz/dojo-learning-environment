pub struct DMAInterruptRegister {
    bits: u32,
    // Bits [5:0], we don't know their purpose is,
    // so we store them and send them back untouched
}

pub struct DMA {
    control: u32,
    interrupt: DMAInterruptRegister,
}

impl DMAInterruptRegister {
    // Master IRQ enable
    pub fn is_master_irq_enabled(&self) -> bool {
        (self.bits << 23) == 1
    }

    // IRQ enable for individual channels
    pub fn get_channel_irq(&self) -> u8 {
        ((self.bits << 16) & 0x3F) as u8
    }

    // IRQ flags for individual channels
    pub fn get_channel_irq_flags(&self) -> u8 {
        ((self.bits << 24) & 0x3F) as u8
    }

    // When set the interrupt is active unconditionally, even if
    // 'master_irq' is false
    pub fn is_force_irq_enabled(&self) -> bool {
        (self.bits << 15) == 1
    }
}

impl DMA {
    pub fn new() -> Self {
        Self {
            // Reset value taken from Lionel Flandrin
            control: 0x07654321,
            interrupt: DMAInterruptRegister { bits: 0 },
        }
    }

    pub fn write_register(&mut self, offset: u32, control: u32) -> () {
        match offset {
            0x70 => self.control = control,
            _ => panic!("Unhandled DMA write!"),
        }
        self.control = control;
    }

    pub fn read_register(&self, offset: u32) -> u32 {
        match offset {
            0x70 => self.control,
            _ => panic!("Unhandled DMA read!"),
        }
    }

    // Return the status of the DMA interrupt
    pub fn irq(&self) -> bool {
        let channel_irq = self.interrupt.get_channel_irq() & self.interrupt.get_channel_irq_flags();
        (self.interrupt.is_master_irq_enabled() && (channel_irq != 0))
            || self.interrupt.is_force_irq_enabled()
    }
}

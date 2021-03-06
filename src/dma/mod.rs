#![allow(dead_code)]

use board;
use alloc::rc::Rc;
use core::cell::RefCell;
use dma::detail::Dma;

mod detail;

const FIFO_SIZE: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    StreamNotReady,

    TransactionCountNotAMultipleOf(u16),
    UnalignedMemoryAddress,
    UnalignedPeripheralAddress,
    CannotUseMemoryToMemoryTransferWithCircularMode,
    CannotUseMemoryToMemoryTransferWithDirectMode,
    MemoryAccessWouldCrossOneKilobyteBoundary,
    PeripheralAccessWouldCrossOneKilobyteBoundary,
    InvalidFifoThresholdMemoryBurstCombination,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stream {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Channel {
    C0 = 0b000,
    C1 = 0b001,
    C2 = 0b010,
    C3 = 0b011,
    C4 = 0b100,
    C5 = 0b101,
    C6 = 0b110,
    C7 = 0b111,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BurstMode {
    SingleTransfer = 0b00,
    Incremental4 = 0b01,
    Incremental8 = 0b10,
    Incremental16 = 0b11,
}

impl BurstMode {
    pub fn get_size(&self) -> u32 {
        match *self {
            BurstMode::SingleTransfer => 1,
            _ => 1 << (*self as u32 + 1)
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryIndex {
    M0 = 0,
    M1 = 1,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoubleBufferingMode {
    Disable,
    UseSecondBuffer(*mut u8),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriorityLevel {
    Low = 0b00,
    Medium = 0b01,
    High = 0b10,
    VeryHigh = 0b11,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeripheralIncrementOffsetSize {
    UsePSize = 0,
    Force32Bit = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Width {
    Byte = 0b00,
    HalfWord = 0b01,
    Word = 0b10,
}

impl Width {
    pub fn get_size(&self) -> u32 {
        1 << *self as u32
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IncrementMode {
    Fixed = 0,
    Increment = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CircularMode {
    Disable = 0,
    Enable = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    PeripheralToMemory = 0b00,
    MemoryToPeripheral = 0b01,
    MemoryToMemory = 0b10,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlowContoller {
    DMA = 0,
    Peripheral = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InterruptControl {
    Disable = 0,
    Enable = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InterruptState {
    NotRaised = 0,
    Raised = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamControl {
    Disable = 0,
    Enable = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FifoStatus {
    FirstQuarter = 0b000,  //  0 < fifo_level < 1/4
    SecondQuarter = 0b001, // 1/4 ≤ fifo_level < 1/2
    ThirdQuarter = 0b010,  // 1/2 ≤ fifo_level < 3/4
    FourthQuarter = 0b011, // 3/4 ≤ fifo_level < full
    Empty = 0b100,
    Full = 0b101,
}

// Yes, this one is inverted
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DirectMode {
    Enable = 0,
    Disable = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FifoThreshold {
    Quarter = 0b00,
    Half = 0b01,
    ThreeQuarter = 0b10,
    Full = 0b11,
}

impl FifoThreshold {
    pub fn get_numerator(&self) -> u32 {
        match *self {
            FifoThreshold::Quarter => 1,
            FifoThreshold::Half => 2,
            FifoThreshold::ThreeQuarter => 3,
            FifoThreshold::Full => 4,
        }
    }

    pub fn get_denominator(&self) -> u32 {
        4
    }
}

#[derive(Debug)]
pub struct DmaTransferNode {
    pub address: *mut u8,
    pub burst_mode: BurstMode,
    pub increment_mode: IncrementMode,
    pub transaction_width: Width,
}

pub type DmaManagerRc = Rc<DmaManagerRefCell>;
type DmaManagerRefCell = RefCell<DmaManager>;

pub struct DmaTransfer {
    pub dma: DmaManagerRc,
    pub stream: Stream,
    pub channel: Channel,
    pub priority: PriorityLevel,
    pub direction: Direction,
    pub circular_mode: CircularMode,
    pub double_buffering_mode: DoubleBufferingMode,
    pub flow_controller: FlowContoller,
    pub peripheral_increment_offset_size: PeripheralIncrementOffsetSize,
    pub peripheral: DmaTransferNode,
    pub memory: DmaTransferNode,
    pub transaction_count: u16,
    pub direct_mode: DirectMode,
    pub fifo_threshold: FifoThreshold,
    pub interrupt_transfer_complete: InterruptControl,
    pub interrupt_half_transfer: InterruptControl,
    pub interrupt_transfer_error: InterruptControl,
    pub interrupt_direct_mode_error: InterruptControl,
    pub interrupt_fifo: InterruptControl,
}

impl DmaTransfer {
    pub fn new(dma: DmaManagerRc, stream: Stream, channel: Channel, direction: Direction, peripheral: DmaTransferNode, memory: DmaTransferNode, transaction_count: u16) -> DmaTransfer {
        let pwidth = peripheral.transaction_width.get_size();
        DmaTransfer {
            dma: dma,
            stream: stream,
            channel: channel,
            priority: PriorityLevel::Medium,
            direction: direction,
            circular_mode: CircularMode::Disable,
            double_buffering_mode: DoubleBufferingMode::Disable,
            flow_controller: FlowContoller::DMA,
            peripheral_increment_offset_size: PeripheralIncrementOffsetSize::UsePSize,
            peripheral: peripheral,
            memory: memory,
            transaction_count: transaction_count,
            direct_mode: if transaction_count as u32 * pwidth >= FIFO_SIZE {
                    DirectMode::Disable
                } else {
                    DirectMode::Enable
                },
            fifo_threshold: FifoThreshold::Full,
            interrupt_transfer_complete: InterruptControl::Enable,
            interrupt_half_transfer: InterruptControl::Disable,
            interrupt_transfer_error: InterruptControl::Enable,
            interrupt_direct_mode_error: InterruptControl::Enable,
            interrupt_fifo: InterruptControl::Enable,
        }
    }

    pub fn is_valid(&self) -> Option<Error> {
        let apply_circular_mode_limitations = self.circular_mode == CircularMode::Enable || self.double_buffering_mode != DoubleBufferingMode::Disable;
        let mwidth = self.memory.transaction_width.get_size();
        let pwidth = match self.peripheral_increment_offset_size {
            PeripheralIncrementOffsetSize::Force32Bit => 4,
            PeripheralIncrementOffsetSize::UsePSize => self.peripheral.transaction_width.get_size(),
        };
        let mburst_size = self.memory.burst_mode.get_size() * mwidth;
        let pburst_size = self.peripheral.burst_mode.get_size() * pwidth;
        let mcount_factor = (mburst_size / pwidth) as u16;
        let pcount_factor = pburst_size as u16;
        let mdata_before_first_kb_boundary = 1024 - (self.memory.address as u32 % 1024);
        let pdata_before_first_kb_boundary = 1024 - (self.peripheral.address as u32 % 1024);
        let mdata_size = mwidth * match self.memory.increment_mode {
             IncrementMode::Increment => self.transaction_count as u32,
             IncrementMode::Fixed => 1,
        };
        let pdata_size = pwidth * match self.peripheral.increment_mode {
             IncrementMode::Increment => self.transaction_count as u32,
             IncrementMode::Fixed => 1,
        };

        if mcount_factor == 0 || self.transaction_count % mcount_factor != 0 {
            Some(Error::TransactionCountNotAMultipleOf(mcount_factor))
        } else if self.transaction_count % pcount_factor != 0 {
            Some(Error::TransactionCountNotAMultipleOf(pcount_factor))
        } else if self.peripheral.address as u32 % self.peripheral.transaction_width.get_size() != 0 {
            Some(Error::UnalignedPeripheralAddress)
        } else if self.memory.address as u32 % self.memory.transaction_width.get_size() != 0 {
            Some(Error::UnalignedMemoryAddress)
        } else if apply_circular_mode_limitations && self.direction == Direction::MemoryToMemory {
            Some(Error::CannotUseMemoryToMemoryTransferWithCircularMode)
        } else if self.direct_mode == DirectMode::Enable && self.direction == Direction::MemoryToMemory {
            Some(Error::CannotUseMemoryToMemoryTransferWithDirectMode)
        } else if mdata_before_first_kb_boundary > mdata_size && mdata_before_first_kb_boundary % mburst_size != 0 {
            Some(Error::MemoryAccessWouldCrossOneKilobyteBoundary)
        } else if pdata_before_first_kb_boundary > pdata_size && pdata_before_first_kb_boundary % pburst_size != 0 {
            Some(Error::PeripheralAccessWouldCrossOneKilobyteBoundary)
        } else if (self.fifo_threshold.get_numerator() * FIFO_SIZE) % (self.fifo_threshold.get_denominator() * mburst_size) != 0 {
            Some(Error::InvalidFifoThresholdMemoryBurstCombination)
        } else {
            None
        }
    }

    pub fn is_ready(&self) -> bool {
        !self.is_active()
    }

    pub fn is_running(&self) -> bool {
        self.dma.borrow().controller.sxcr_en(self.stream) == StreamControl::Enable
    }

    pub fn is_finished(&self) -> bool {
        self.dma.borrow().controller.tcif(self.stream) == InterruptState::Raised
    }

    pub fn is_error(&self) -> bool {
        self.is_transfer_error() || self.is_direct_mode_error()
    }

    pub fn is_transfer_error(&self) -> bool {
        self.dma.borrow().controller.teif(self.stream) == InterruptState::Raised
    }

    pub fn is_direct_mode_error(&self) -> bool {
        self.dma.borrow().controller.dmeif(self.stream) == InterruptState::Raised
    }

    pub fn is_active(&self) -> bool {
        self.is_running() && !self.is_finished() && !self.is_error()
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let result = self.is_valid();

        if result.is_none() {
            if self.is_ready() {
                self.configure();

                self.dma.borrow_mut().controller.set_sxcr_en(self.stream, StreamControl::Enable);

                Ok(())
            } else {
                Err(Error::StreamNotReady)
            }
        } else {
            Err(result.unwrap())
        }
    }

    pub fn stop(&mut self) {
        self.dma.borrow_mut().controller.set_sxcr_en(self.stream, StreamControl::Disable);
    }

    pub fn wait(&self) -> bool {
        while self.is_active() {};

        !self.is_error()
    }

    pub fn execute(&mut self) -> Result<bool, Error> {
        match self.start() {
            Ok(_) => Ok({
                let result = self.wait();
                self.stop();

                result
            }),
            Err(x) => Err(x),
        }
    }

    fn configure(&mut self) {
        self.dma.borrow_mut().controller.clear_htif(self.stream);
        self.dma.borrow_mut().controller.clear_tcif(self.stream);
        self.dma.borrow_mut().controller.clear_teif(self.stream);
        self.dma.borrow_mut().controller.clear_feif(self.stream);
        self.dma.borrow_mut().controller.clear_dmeif(self.stream);

        self.dma.borrow_mut().controller.set_sxcr_channel(self.stream, self.channel);
        self.dma.borrow_mut().controller.set_sxcr_pl(self.stream, self.priority);
        self.dma.borrow_mut().controller.set_sxcr_dir(self.stream, self.direction);
        self.dma.borrow_mut().controller.set_sxcr_circ(self.stream, self.circular_mode);
        self.dma.borrow_mut().controller.set_sxcr_dbm(self.stream, self.double_buffering_mode);
        self.dma.borrow_mut().controller.set_sxcr_pfctrl(self.stream, self.flow_controller);
        self.dma.borrow_mut().controller.set_sxcr_psize(self.stream, self.peripheral.transaction_width);
        self.dma.borrow_mut().controller.set_sxcr_pinc(self.stream, self.peripheral.increment_mode);
        self.dma.borrow_mut().controller.set_sxcr_pburst(self.stream, self.peripheral.burst_mode);
        self.dma.borrow_mut().controller.set_sxcr_pincos(self.stream, self.peripheral_increment_offset_size);
        self.dma.borrow_mut().controller.set_sxpar(self.stream, self.peripheral.address);
        self.dma.borrow_mut().controller.set_sxcr_msize(self.stream, self.memory.transaction_width);
        self.dma.borrow_mut().controller.set_sxcr_minc(self.stream, self.memory.increment_mode);
        self.dma.borrow_mut().controller.set_sxcr_mburst(self.stream, self.memory.burst_mode);
        self.dma.borrow_mut().controller.set_sxmxar(self.stream, MemoryIndex::M0, self.memory.address);
        self.dma.borrow_mut().controller.set_sxndtr(self.stream, self.transaction_count);
        self.dma.borrow_mut().controller.set_sxfcr_dmdis(self.stream, self.direct_mode);
        self.dma.borrow_mut().controller.set_sxfcr_fth(self.stream, self.fifo_threshold);

        self.dma.borrow_mut().controller.set_sxcr_tcie(self.stream, self.interrupt_transfer_complete);
        self.dma.borrow_mut().controller.set_sxcr_htie(self.stream, self.interrupt_half_transfer);
        self.dma.borrow_mut().controller.set_sxcr_teie(self.stream, self.interrupt_transfer_error);
        self.dma.borrow_mut().controller.set_sxcr_dmeie(self.stream, self.interrupt_direct_mode_error);
        self.dma.borrow_mut().controller.set_sxfcr_feie(self.stream, self.interrupt_fifo);
    }
}

pub struct DmaManager {
    controller: Dma
}

impl DmaManager {
    pub fn init_dma1(dma_1: &'static mut board::dma::Dma, rcc: &mut board::rcc::Rcc) -> DmaManagerRc {
       // enable DMA1 clock and wait until the clock is up
        rcc.ahb1enr.update(|r| r.set_dma1en(true));
        loop {
            if rcc.ahb1enr.read().dma1en() {break;};
        }

        DmaManagerRc::new(
            DmaManagerRefCell::new(
                DmaManager {
                    controller: Dma::init(dma_1),
                }
            )
        )
    }

    pub fn init_dma2(dma_2: &'static mut board::dma::Dma, rcc: &mut board::rcc::Rcc) -> DmaManagerRc {
       // enable DMA1 clock and wait until the clock is up
        rcc.ahb1enr.update(|r| r.set_dma2en(true));
        loop {
            if rcc.ahb1enr.read().dma2en() {break;};
        }

        DmaManagerRc::new(
            DmaManagerRefCell::new(
                DmaManager {
                    controller: Dma::init(dma_2),
                }
            )
        )
    }
}
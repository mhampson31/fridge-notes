#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
#[cfg(feature = "blocking")]
use embedded_hal::spi::SpiDevice;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::spi::SpiDevice;

#[cfg(feature = "blocking")]
use embedded_hal::digital::InputPin as Wait;
use embedded_hal::digital::{InputPin, OutputPin};
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::digital::Wait;

use epdsi::{
    bus::{EpdBusError, SpiBusWrapper},
    traits::{ColorChannel, ColorMode, EpdController, EpdPanel},
};

/// Physical panel driver specification for Xteink X3 display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct XTEinkX3 {}

impl EpdPanel for XTEinkX3 {
    const WIDTH: u32 = 600;
    const HEIGHT: u32 = 792;
    const COLOR_MODE: ColorMode = ColorMode::BlackWhite;
}

/* Controller */

pub struct UC8279d {
    width: u32,
    height: u32,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<SPI, DC, RST, BUSY> EpdController<SpiBusWrapper<SPI, DC, RST, BUSY>> for UC8279d
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin + Wait,
{
    type Error = EpdBusError<SPI::Error, DC::Error, RST::Error, BUSY::Error>;

    async fn init_sequence<DELAY: DelayNs>(
        &mut self,
        bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        delay: &mut DELAY,
    ) -> Result<(), Self::Error> {
        bus.hard_reset(delay, 10).await?;
        // `true` means BUSY is high while the panel works. Check your panel's polarity.
        bus.wait_busy(true).await?;
        bus.send_command_with_data(0x01, &[(self.height - 1) as u8, 0x00])
            .await
    }

    async fn set_window(
        &mut self,
        _bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _x_start: u32,
        _y_start: u32,
        _x_end: u32,
        _y_end: u32,
    ) -> Result<(), Self::Error> {
        Ok(()) // full-frame streaming only
    }

    async fn set_cursor(
        &mut self,
        _bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _x: u32,
        _y: u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn write_frame(
        &mut self,
        bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _channel: ColorChannel,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        bus.send_command_with_data(0x24, data).await
    }

    async fn write_frame_pattern(
        &mut self,
        bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _channel: ColorChannel,
        byte: u8,
        count: usize,
    ) -> Result<(), Self::Error> {
        bus.send_command(0x24).await?;
        bus.send_data_repeated(byte, count).await
    }

    async fn trigger_refresh<DELAY: DelayNs>(
        &mut self,
        bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _delay: &mut DELAY,
    ) -> Result<(), Self::Error> {
        bus.send_command(0x20).await?;
        bus.wait_busy(true).await
    }

    async fn sleep<DELAY: DelayNs>(
        &mut self,
        bus: &mut SpiBusWrapper<SPI, DC, RST, BUSY>,
        _delay: &mut DELAY,
    ) -> Result<(), Self::Error> {
        bus.send_command_with_data(0x10, &[0x01]).await
    }
}

#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use stm32f4xx_hal::{
        gpio::{Output, PC13},
        otg_fs::{UsbBus, UsbBusType, USB},
        pac,
        prelude::*,
        timer::MonoTimerUs,
    };

    use stm32_device_signature::device_id_hex;
    use usb_device::prelude::*;
    use usb_io::class::UsbIoClass;

    #[shared]
    struct Shared {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_io: UsbIoClass<'static, UsbBusType>,
    }

    #[local]
    struct Local {
        led: PC13<Output>,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        static mut EP_MEMORY: [u32; 1024] = [0; 1024];
        static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<UsbBusType>> = None;

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        // Setup system clocks
        let hse = 25.MHz();
        let sysclk = 84.MHz();
        let clocks = rcc
            .cfgr
            .use_hse(hse)
            .sysclk(sysclk)
            .require_pll48clk()
            .freeze();

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();
        let led = gpioc.pc13.into_push_pull_output();

        let mono = dp.TIM2.monotonic_us(&clocks);
        tick::spawn().ok();

        // *** Begin USB setup ***
        let usb = USB {
            usb_global: dp.OTG_FS_GLOBAL,
            usb_device: dp.OTG_FS_DEVICE,
            usb_pwrclk: dp.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate(),
            pin_dp: gpioa.pa12.into_alternate(),
            hclk: clocks.hclk(),
        };
        unsafe {
            USB_BUS.replace(UsbBus::new(usb, &mut EP_MEMORY));
        }

        let usb_io = UsbIoClass::new(unsafe { USB_BUS.as_ref().unwrap() });
        let usb_dev =
            usb_io.make_device(unsafe { USB_BUS.as_ref().unwrap() }, Some(device_id_hex()));
        (
            Shared { usb_dev, usb_io },
            Local { led },
            init::Monotonics(mono),
        )
    }

    #[task(local = [led])]
    fn tick(ctx: tick::Context) {
        tick::spawn_after(1.secs()).ok();
        ctx.local.led.toggle();
    }

    #[task(binds=OTG_FS, shared=[usb_dev, usb_io])]
    fn usb_fs(cx: usb_fs::Context) {
        let usb_fs::SharedResources {
            mut usb_dev,
            mut usb_io,
        } = cx.shared;

        (&mut usb_dev, &mut usb_io).lock(|usb_dev, usb_io| {
            usb_dev.poll(&mut [usb_io]);
        });
    }
}

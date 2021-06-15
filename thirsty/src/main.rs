#![no_std]
#![no_main]

// you can put a breakpoint on `rust_begin_unwind` to catch panics
use panic_halt as _;

use rtic::app;

use core::fmt::Write;
use embedded_hal::digital::v2::OutputPin;
use stm32f1xx_hal::gpio::{gpioc::PC13, Output, PushPull, State, Alternate, OpenDrain, Analog};
use stm32f1xx_hal::{pac, delay};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::timer::{CountDownTimer, Event, Timer};
use stm32f1xx_hal::adc::Adc;
use stm32f1xx_hal::i2c::BlockingI2c;
use stm32f1xx_hal::pac::{ADC1, I2C2, ADC2};
use stm32f1xx_hal::i2c::{Mode, DutyCycle};
use ssd1306::{I2CDIBuilder, Builder};
use ssd1306::mode::GraphicsMode;
use ssd1306::prelude::I2CInterface;
use ssd1306::displaysize::DisplaySize128x64;
use stm32f1xx_hal::gpio::gpiob::{PB10, PB11, PB5, PB0, PB1};
use stm32f1xx_hal::delay::Delay;
use e_write_buffer::WriteBuffer;
use embedded_graphics::fonts::{Text, Font6x8};
use embedded_graphics::geometry::Point;
use embedded_graphics::style::{TextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::drawable::Drawable;
use cortex_m_semihosting::hprintln;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        led: PC13<Output<PushPull>>,
        timer_handler: CountDownTimer<pac::TIM1>,
        battery_adc: Adc<ADC1>,
        moisture_adc: Adc<ADC2>,
        display: GraphicsMode<I2CInterface<BlockingI2c<I2C2,(PB10<Alternate<OpenDrain>>, PB11<Alternate<OpenDrain>>)>>, DisplaySize128x64>,
        delay: Delay,
        dht_pin: PB5<Output<OpenDrain>>,
        ch0: PB0<Analog>,
        ch1: PB1<Analog>,

        #[init(false)]
        led_state: bool,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        hprintln!("init").unwrap();
        // Take ownership over the raw flash and rcc devices and convert them into the corresponding
        // HAL structs
        let mut flash = cx.device.FLASH.constrain();
        let mut rcc = cx.device.RCC.constrain();

        // Freeze the configuration of all the clocks in the system and store the frozen frequencies
        // in `clocks`
        let clocks = rcc.cfgr.adcclk(2.mhz()).use_hse(8.mhz()).freeze(&mut flash.acr);

        // Acquire the GPIO peripherals
        let mut gpioc = cx.device.GPIOC.split(&mut rcc.apb2);
        let mut gpiob = cx.device.GPIOB.split(&mut rcc.apb2);

        // Setup ADC
        let battery_adc = Adc::adc1(cx.device.ADC1, &mut rcc.apb2, clocks);
        let moisture_adc = Adc::adc2(cx.device.ADC2, &mut rcc.apb2, clocks);
        // Configure pb0,pb1 as an analog input
        let ch0 = gpiob.pb0.into_analog(&mut gpiob.crl);
        let ch1 = gpiob.pb1.into_analog(&mut gpiob.crl);

        // Setup display
        let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);

        let i2c = BlockingI2c::i2c2(
            cx.device.I2C2,
            (scl, sda),
            Mode::Fast {
                frequency: 400_000.hz(),
                duty_cycle: DutyCycle::Ratio2to1,
            },
            clocks,
            &mut rcc.apb1,
            1000,
            10,
            1000,
            1000,
        );
        let interface = I2CDIBuilder::new().init(i2c);
        let mut display: GraphicsMode<_, _> = Builder::new().connect(interface).into();
        display.init().unwrap();

        // This is used by `dht-sensor` to wait for signals
        let delay = delay::Delay::new(cx.core.SYST, clocks);
        let mut dht_pin = gpiob.pb5.into_open_drain_output(&mut gpiob.crl);
        dht_pin.set_high().unwrap();

        // Configure gpio C pin 13 as a push-pull output. The `crh` register is passed to the
        // function in order to configure the port. For pins 0-7, crl should be passed instead
        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, State::High);

        // Configure the syst timer to trigger an update every second and enables interrupt
        let mut timer =
            Timer::tim1(cx.device.TIM1, &clocks, &mut rcc.apb2).start_count_down(1.hz());
        timer.listen(Event::Update);

        hprintln!("end init").unwrap();
        // Init the static resources to use them later through RTIC
        init::LateResources {
            led,
            battery_adc,
            moisture_adc,
            display,
            dht_pin,
            delay,
            timer_handler: timer,
            ch0,
            ch1,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        hprintln!("idle").unwrap();
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = TIM1_UP, priority = 1, resources = [led, timer_handler, led_state, battery_adc, moisture_adc, display, dht_pin, delay, ch0, ch1])]
    fn tick(cx: tick::Context) {
        hprintln!("tick").unwrap();
        let mut buffer: WriteBuffer<20> = WriteBuffer::new();

        if *cx.resources.led_state {
            cx.resources.led.set_high().unwrap();
            *cx.resources.led_state = false;
        } else {
            cx.resources.led.set_low().unwrap();
            *cx.resources.led_state = true;
        }

        let battery: u16 = cx.resources.battery_adc.read(cx.resources.ch0).unwrap();
        let moisture: u16 = cx.resources.moisture_adc.read(cx.resources.ch1).unwrap();

        let text_style = TextStyleBuilder::new(Font6x8)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        write!(buffer, "Moisture {:>9}", moisture).unwrap();
        Text::new(&buffer.as_str().unwrap(), Point::zero())
            .into_styled(text_style)
            .draw(cx.resources.display)
            .unwrap();
        buffer.reset();

        write!(buffer, "Moisture {:>9}", battery).unwrap();
        Text::new(&buffer.as_str().unwrap(), Point::new(0, 12))
            .into_styled(text_style)
            .draw(cx.resources.display)
            .unwrap();
        buffer.reset();

        // Clears the update flag
        cx.resources.timer_handler.clear_update_interrupt_flag();
    }
};
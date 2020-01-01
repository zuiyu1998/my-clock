#![no_main]
#![no_std]

#[cfg(not(test))]
extern crate panic_semihosting;

use epd_waveshare::prelude::*;
use portable::datetime::DateTime;
use portable::{ button, datetime, ui};

use rtfm::app;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::{delay, gpio,  rtc, spi, stm32, timer};

type Button0Pin = gpio::gpioa::PA6<gpio::Input<gpio::PullUp>>;
type Button1Pin = gpio::gpioa::PA7<gpio::Input<gpio::PullUp>>;
type Button2Pin = gpio::gpiob::PB0<gpio::Input<gpio::PullUp>>;
type Button3Pin = gpio::gpiob::PB1<gpio::Input<gpio::PullUp>>;
type Spi = spi::Spi<
    stm32::SPI2,
    (
        gpio::gpiob::PB13<gpio::Alternate<gpio::PushPull>>,
        gpio::gpiob::PB14<gpio::Input<gpio::Floating>>,
        gpio::gpiob::PB15<gpio::Alternate<gpio::PushPull>>,
    ),
>;
type EPaperDisplay = epd_waveshare::epd2in9::EPD2in9<
    Spi,
    gpio::gpiob::PB12<gpio::Output<gpio::PushPull>>, // cs/nss
    gpio::gpioa::PA10<gpio::Input<gpio::Floating>>,  // busy
    gpio::gpioa::PA8<gpio::Output<gpio::PushPull>>,  // dc
    gpio::gpioa::PA9<gpio::Output<gpio::PushPull>>,  // rst
>;

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut RTC_DEV: rtc::Rtc = ();
    static mut BUTTON0: button::Button<Button0Pin> = ();
    static mut BUTTON1: button::Button<Button1Pin> = ();
    static mut BUTTON2: button::Button<Button2Pin> = ();
    static mut BUTTON3: button::Button<Button3Pin> = ();
    static mut DISPLAY: EPaperDisplay = ();
    static mut SPI: Spi = ();
    static mut UI: ui::Model = ();
    static mut FULL_UPDATE: bool = false;
    static mut TIMER: stm32f1xx_hal::timer::Timer<stm32::TIM3> = ();

    #[init(spawn = [msg])]
    fn init() -> init::LateResources {
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();
       
        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(72.mhz())
            .pclk1(36.mhz())
            .freeze(&mut flash.acr);
        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);

        let button0_pin = gpioa.pa6.into_pull_up_input(&mut gpioa.crl);
        let button1_pin = gpioa.pa7.into_pull_up_input(&mut gpioa.crl);
        let button2_pin = gpiob.pb0.into_pull_up_input(&mut gpiob.crl);
        let button3_pin = gpiob.pb1.into_pull_up_input(&mut gpiob.crl);

        let mut timer = timer::Timer::tim3(device.TIM3, 1.khz(), clocks, &mut rcc.apb1);
        timer.listen(timer::Event::Update);

        let mut backup_domain = rcc
            .bkp
            .constrain(device.BKP, &mut rcc.apb1, &mut device.PWR);
        let mut rtc = rtc::Rtc::rtc(device.RTC, &mut backup_domain);
        if rtc.seconds() < 100 {
            let today = DateTime {
                year: 2018,
                month: 9,
                day: 1,
                hour: 23,
                min: 59,
                sec: 40,
                day_of_week: datetime::DayOfWeek::Wednesday,
            };
            if let Some(epoch) = today.to_epoch() {
                rtc.set_seconds(epoch);
            }
        }
        rtc.listen_seconds();

        let mut delay = delay::Delay::new(core.SYST, clocks);

        let sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
        let miso = gpiob.pb14;
        let mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);
        let mut spi = spi::Spi::spi2(
            device.SPI2,
            (sck, miso, mosi),
            epd_waveshare::SPI_MODE,
            4.mhz(),
            clocks,
            &mut rcc.apb1,
        );
        let mut il3820 = epd_waveshare::epd2in9::EPD2in9::new(
            &mut spi,
            gpiob.pb12.into_push_pull_output(&mut gpiob.crh),
            gpioa.pa10.into_floating_input(&mut gpioa.crh),
            gpioa.pa8.into_push_pull_output(&mut gpioa.crh),
            gpioa.pa9.into_push_pull_output(&mut gpioa.crh),
            &mut delay,
        )
        .unwrap();
        il3820.set_lut(&mut spi, Some(RefreshLUT::QUICK)).unwrap();
        il3820.clear_frame(&mut spi).unwrap();

        init::LateResources {
            RTC_DEV: rtc,
            BUTTON0: button::Button::new(button0_pin),
            BUTTON1: button::Button::new(button1_pin),
            BUTTON2: button::Button::new(button2_pin),
            BUTTON3: button::Button::new(button3_pin),
            DISPLAY: il3820,
            SPI: spi,
            UI: ui::Model::init(),
            TIMER: timer,
        }
    }

    #[interrupt(priority = 4, spawn = [msg], resources = [BUTTON0, BUTTON1, BUTTON2, BUTTON3,TIMER])]
    fn TIM3() {
        resources.TIMER.clear_update_interrupt_flag();

        if let button::Event::Pressed = resources.BUTTON0.poll() {
            spawn.msg(ui::Msg::ButtonCancel).unwrap();
        }
        if let button::Event::Pressed = resources.BUTTON1.poll() {
            spawn.msg(ui::Msg::ButtonMinus).unwrap();
        }
        if let button::Event::Pressed = resources.BUTTON2.poll() {
            spawn.msg(ui::Msg::ButtonPlus).unwrap();
        }
        if let button::Event::Pressed = resources.BUTTON3.poll() {
            spawn.msg(ui::Msg::ButtonOk).unwrap();
        }
    }

    #[interrupt(priority = 3, spawn = [msg], resources = [RTC_DEV])]
    fn RTC() {
        resources.RTC_DEV.clear_second_flag();

        let datetime = DateTime::new(resources.RTC_DEV.seconds());

        spawn.msg(ui::Msg::DateTime(datetime)).unwrap();
    }

    #[task(priority = 2, capacity = 16, spawn = [msg], resources = [UI, RTC_DEV, FULL_UPDATE])]
    fn msg(msg: ui::Msg) {
        use crate::ui::Cmd::*;
        for cmd in resources.UI.update(msg) {
            match cmd {
                UpdateRtc(dt) => {
                    if let Some(epoch) = dt.to_epoch() {
                        resources.RTC_DEV.lock(|rtc| {
                            let _ = rtc.set_seconds(epoch);
                        });
                        spawn.msg(ui::Msg::DateTime(dt)).unwrap();
                    }
                }
                FullUpdate => *resources.FULL_UPDATE = true,
            }
        }
        rtfm::pend(stm32::Interrupt::EXTI1);
    }

    #[interrupt(priority = 1, resources = [UI, DISPLAY, SPI, FULL_UPDATE])]
    fn EXTI1() {
        let model = resources.UI.lock(|model| model.clone());
        let display = model.view();
        let full_update = resources
            .FULL_UPDATE
            .lock(|fu| core::mem::replace(&mut *fu, false));
        if full_update {
            resources
                .DISPLAY
                .set_lut(&mut *resources.SPI, Some(RefreshLUT::FULL))
                .unwrap();
        }

        resources
            .DISPLAY
            .update_frame(&mut *resources.SPI, &display.buffer())
            .unwrap();
        resources
            .DISPLAY
            .display_frame(&mut *resources.SPI)
            .unwrap();

        if full_update {
            // partial/quick refresh needs only be set when a full update was run before
            resources
                .DISPLAY
                .set_lut(&mut *resources.SPI, Some(RefreshLUT::QUICK))
                .unwrap();
        }
    }

    // Interrupt handlers used to dispatch software tasks
    extern "C" {
        fn EXTI2();
    }
};


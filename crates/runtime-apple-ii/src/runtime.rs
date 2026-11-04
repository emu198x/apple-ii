//! Oscillator-driven execution and host delivery.
use crate::{
    input::{Input, KEYBOARD, apply_input_event},
    profiles,
};
use emu198x_apple_disk_ii::DiskII;
use emu198x_machine_apple_ii_rev0_ntsc::{AppleII, COLOUR_WIDTH, HEIGHT};
use emu198x_shell::{
    AudioPacket, CapabilitySet, FirmwareSet, FramePacket, HostIo, MachineCore, MachineError,
    MachineProfile, MachineTime, MediaSet, PixelFormat, ResetKind, RunResult, StopReason,
};
use format198x_apple_ii_woz::Woz;
/// One concrete original-board session. Firmware/media bytes are caller supplied.
pub struct AppleIIRuntime {
    pub(crate) machine: AppleII,
    pub(crate) input: Input,
    pub(crate) images: [Option<Woz>; 2],
    profile: MachineProfile,
    rgba: Vec<u8>,
}
fn firmware<const N: usize>(set: &FirmwareSet<'_>, id: &str) -> Result<[u8; N], MachineError> {
    set.image(id)
        .ok_or_else(|| MachineError::MissingFirmware { id: id.into() })?
        .bytes
        .try_into()
        .map_err(|_| MachineError::InvalidFirmware {
            id: id.into(),
            reason: format!("expected {N} bytes"),
        })
}
impl AppleIIRuntime {
    /// Validate all firmware, optionally install a complete P5/P6 pair and press RESET.
    pub fn from_firmware(set: &FirmwareSet<'_>) -> Result<Self, MachineError> {
        let profile = profiles::profile();
        set.validate_for_profile(&profile)?;
        let mut machine = AppleII::new(
            firmware(set, profiles::FIRMWARE[0])?,
            firmware(set, profiles::FIRMWARE[1])?,
        );
        if set.image(profiles::FIRMWARE[2]).is_some() || set.image(profiles::FIRMWARE[3]).is_some()
        {
            let card = DiskII::new(
                firmware(set, profiles::FIRMWARE[2])?,
                firmware(set, profiles::FIRMWARE[3])?,
                emu198x_machine_apple_ii_rev0_ntsc::MASTER_HZ as u32,
            )
            .map_err(|e| MachineError::InvalidRequest {
                reason: e.to_string(),
            })?;
            machine.install_disk_ii(card);
        }
        machine.reset();
        Ok(Self {
            machine,
            input: Input::default(),
            images: [None, None],
            profile,
            rgba: vec![0; COLOUR_WIDTH * HEIGHT * 4],
        })
    }
    /// Read the live board without changing its bus or time.
    pub fn machine(&self) -> &AppleII {
        &self.machine
    }
    pub(crate) fn update_rgba_framebuffer(&mut self) {
        for (rgba, pixel) in self
            .rgba
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.machine.colour_framebuffer())
        {
            let [_, r, g, b] = pixel.to_be_bytes();
            rgba.copy_from_slice(&[r, g, b, 255]);
        }
    }
    fn present(&mut self, host: &mut HostIo<'_>) -> Result<(), MachineError> {
        self.update_rgba_framebuffer();
        host.frame_sink.push_frame(FramePacket {
            signal: None,
            timestamp: self.time(),
            format: PixelFormat::Rgba8888,
            width: COLOUR_WIDTH as u32,
            height: HEIGHT as u32,
            palette: None,
            pixels: &self.rgba,
        })
    }
    pub(crate) fn tick(&mut self) -> emu198x_machine_apple_ii_rev0_ntsc::TickOutput {
        if !self.machine.key_pending()
            && let Some(byte) = self.input.queue.pop_front()
        {
            self.machine.key(byte);
        }
        self.machine.tick()
    }
    fn run_window(
        &mut self,
        target: u64,
        one_frame: bool,
        host: &mut HostIo<'_>,
    ) -> Result<RunResult, MachineError> {
        if target <= self.machine.master_clock() {
            return Ok(RunResult::new(self.time(), StopReason::ReachedTarget));
        }
        for event in host.input_events {
            apply_input_event(&mut self.input, event)?;
        }
        let mut audio = Vec::with_capacity(803);
        let mut last_present = None;
        while self.machine.master_clock() < target {
            let out = self.tick();
            if let Some(sample) = out.audio_sample {
                audio.push(sample);
            }
            if out.frame_complete {
                self.present(host)?;
                last_present = Some(self.time());
                host.audio_sink.push_audio(AudioPacket {
                    timestamp: self.time(),
                    sample_rate: 48_000,
                    channels: 1,
                    samples: &audio,
                })?;
                audio.clear();
                if one_frame {
                    return Ok(RunResult::new(self.time(), StopReason::ReachedTarget));
                }
            }
        }
        if last_present != Some(self.time()) {
            self.present(host)?;
        }
        if !audio.is_empty() {
            host.audio_sink.push_audio(AudioPacket {
                timestamp: self.time(),
                sample_rate: 48_000,
                channels: 1,
                samples: &audio,
            })?;
        }
        Ok(RunResult::new(
            self.time(),
            if one_frame {
                StopReason::NoFrame
            } else {
                StopReason::ReachedTarget
            },
        ))
    }
}
impl MachineCore for AppleIIRuntime {
    fn display(&self) -> Option<emu198x_shell::display::Display> {
        // Colour output retains one sample per 14M serializer tick in every
        // mode (docs/hardware.md, Graphics signal and presentation).
        Some(emu198x_shell::display::Display::Television {
            region: emu198x_shell::Region::Ntsc,
            pixel_clock_hz: emu198x_machine_apple_ii_rev0_ntsc::MASTER_HZ as f64,
            lines_per_tv_height: emu198x_shell::display::NTSC_ACTIVE_LINES,
        })
    }
    fn profile(&self) -> &MachineProfile {
        &self.profile
    }
    fn time(&self) -> MachineTime {
        MachineTime::new(self.machine.master_clock())
    }
    fn reset(&mut self, _kind: ResetKind) {
        self.machine.reset();
        self.input = Input::default();
    }
    fn capabilities(&self) -> CapabilitySet {
        self.profile.capabilities.clone()
    }
    fn run_until(
        &mut self,
        target: MachineTime,
        host: &mut HostIo<'_>,
    ) -> Result<RunResult, MachineError> {
        self.run_window(target.get(), false, host)
    }
    fn run_ticks(&mut self, ticks: u64, host: &mut HostIo<'_>) -> Result<RunResult, MachineError> {
        let target = self
            .machine
            .master_clock()
            .checked_add(ticks)
            .ok_or_else(|| MachineError::InvalidRequest {
                reason: "tick count overflows machine time".into(),
            })?;
        self.run_window(target, false, host)
    }
    fn run_frame(&mut self, host: &mut HostIo<'_>) -> Result<RunResult, MachineError> {
        self.run_window(
            self.machine
                .master_clock()
                .saturating_add(profiles::FRAME_TICKS + 1),
            true,
            host,
        )
    }
    fn load_media(&mut self, media: &MediaSet<'_>) -> Result<(), MachineError> {
        self.mount_media(media)
    }
    fn export_media(&self, slot: &str, format: &str) -> Result<Vec<u8>, MachineError> {
        self.export_disk(slot, format)
    }
    fn eject_media(&mut self, slot: &str) -> Result<(), MachineError> {
        self.eject_disk(slot)
    }
    fn snapshot(&self) -> Result<Vec<u8>, MachineError> {
        Err(MachineError::UnsupportedOperation {
            operation: "snapshot",
        })
    }
    fn restore(&mut self, _bytes: &[u8]) -> Result<(), MachineError> {
        Err(MachineError::UnsupportedOperation {
            operation: "restore",
        })
    }
    fn debug_target(&self) -> Option<&dyn emu198x_shell::DebugTarget> {
        Some(self)
    }
    fn debug_target_mut(&mut self) -> Option<&mut dyn emu198x_shell::DebugTarget> {
        Some(self)
    }
    fn keyboard_target(&self) -> Option<&dyn emu198x_shell::KeyboardTarget> {
        Some(&KEYBOARD)
    }
}

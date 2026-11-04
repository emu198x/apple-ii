//! One concrete board; later boards get separate machine owners.
use emu198x_machine_apple_ii_rev0_ntsc::MASTER_HZ;
use emu198x_shell::{
    CapabilitySet, ClockDesc, ClockRate, Family, FirmwareRequirement, MachineId, MachineProfile,
    MediaKind, MediaSlot, ProfileId, Region, WritebackPolicy, known_capability,
};
/// Full native field budget, used only as a pacing hint.
pub const FRAME_TICKS: u64 = 912 * 262;
/// Firmware identifiers accepted by this runtime.
pub const FIRMWARE: [&str; 4] = [
    "apple-ii-rom",
    "apple-ii-characters",
    "disk-ii-p5",
    "disk-ii-p6",
];
/// Profile for the original NTSC revision-0 board, 48 KB fitted.
pub fn profile() -> MachineProfile {
    MachineProfile {
        machine_id: MachineId::from("apple-ii-rev0-ntsc"),
        profile_id: ProfileId::from("apple-ii-rev0-ntsc-48k"),
        display_name: "Apple II (original NTSC revision 0, 48 KB)".into(),
        family: Family::Other,
        region: Region::Ntsc,
        release_year: 1977,
        summary: "Original board with Monitor/Integer BASIC and optional slot-6 Disk II".into(),
        clock: ClockDesc::new("master-crystal", ClockRate::from_hz(MASTER_HZ)),
        firmware: FIRMWARE
            .iter()
            .enumerate()
            .map(|(i, id)| FirmwareRequirement::new(*id, *id, i >= 2))
            .collect(),
        media_slots: ["floppy-0", "floppy-1"]
            .into_iter()
            .map(|id| MediaSlot::new(id, id, MediaKind::Disk, false, WritebackPolicy::SidecarOnly))
            .collect(),
        capabilities: CapabilitySet::with_all(
            ["keyboard-input", "media-export", "audio-output"]
                .into_iter()
                .map(known_capability),
        ),
    }
}

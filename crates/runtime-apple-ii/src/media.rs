//! Archive-safe mounting and export. No host path is opened here.
use crate::AppleIIRuntime;
use emu198x_machine_apple_ii_rev0_ntsc::disk;
use emu198x_shell::{MachineError, MediaKind, MediaSet};
use format198x_apple_ii_woz::Woz;
fn drive(slot: &str) -> Result<usize, MachineError> {
    match slot {
        "floppy-0" => Ok(0),
        "floppy-1" => Ok(1),
        _ => Err(MachineError::UnknownMediaSlot { slot: slot.into() }),
    }
}
fn invalid(slot: &str, reason: impl ToString) -> MachineError {
    MachineError::InvalidMedia {
        slot: slot.into(),
        reason: reason.to_string(),
    }
}
impl AppleIIRuntime {
    fn idle(&self) -> Result<(), MachineError> {
        let card = self
            .machine
            .disk_ii()
            .ok_or_else(|| MachineError::MissingFirmware {
                id: "disk-ii-p5 and disk-ii-p6".into(),
            })?;
        if card.status().motor_on {
            return Err(MachineError::InvalidRequest{reason:"Disk II motor is active; wait for motor-off before replacing, ejecting or exporting media".into()});
        }
        Ok(())
    }
    pub(crate) fn mount_media(&mut self, media: &MediaSet<'_>) -> Result<(), MachineError> {
        if media.images.is_empty() {
            return Ok(());
        }
        self.idle()?;
        let mut prepared = Vec::new();
        for image in &media.images {
            let index = drive(&image.slot)?;
            if image.kind != MediaKind::Disk {
                return Err(MachineError::UnsupportedMediaKind { kind: image.kind });
            }
            let mut woz = Woz::parse(image.bytes).map_err(|e| invalid(&image.slot, e))?;
            if image.writable {
                woz.set_write_protected(false);
            }
            let disk = disk::mount(&woz, image.writable).map_err(|e| invalid(&image.slot, e))?;
            prepared.push((index, woz, disk));
        }
        for (index, woz, disk) in prepared {
            self.machine
                .disk_ii_mut()
                .expect("idle validated card")
                .insert(index, disk)
                .map_err(|e| invalid("Disk II", e))?;
            self.images[index] = Some(woz);
        }
        Ok(())
    }
    pub(crate) fn export_disk(&self, slot: &str, format: &str) -> Result<Vec<u8>, MachineError> {
        let index = drive(slot)?;
        self.idle()?;
        if format != "woz" {
            return Err(MachineError::UnsupportedOperation {
                operation: "export format (expected woz)",
            });
        }
        let image = self.images[index]
            .as_ref()
            .ok_or_else(|| invalid(slot, "drive is empty"))?;
        let disk = self
            .machine
            .disk_ii()
            .and_then(|c| c.disk(index))
            .ok_or_else(|| invalid(slot, "drive is empty"))?;
        disk::export(image, disk).map_err(|e| invalid(slot, e))
    }
    pub(crate) fn eject_disk(&mut self, slot: &str) -> Result<(), MachineError> {
        let index = drive(slot)?;
        self.idle()?;
        self.machine
            .disk_ii_mut()
            .expect("idle validated card")
            .eject(index)
            .map_err(|e| invalid(slot, e))?;
        self.images[index] = None;
        Ok(())
    }
}

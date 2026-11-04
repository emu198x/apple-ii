//! Thin WOZ-to-bitstream adapter. Source images remain caller-owned and immutable.
use emu198x_apple_disk_ii::{Disk, DiskError, Track};
use format198x_apple_ii_woz::Woz;

/// Convert validated WOZ tracks into independent controller media. Sparse track
/// indices are compacted without changing shared quarter-track relationships.
pub fn mount(image: &Woz, writable: bool) -> Result<Disk, DiskError> {
    let mut tracks = Vec::new();
    let mut indices = [255; 160];
    for (index, entry) in indices.iter_mut().enumerate() {
        if let Some((bytes, count)) = image.track(index) {
            *entry = tracks.len() as u8;
            tracks.push(Track::new(bytes.to_vec(), count)?);
        }
    }
    let map = image.track_map().map(|index| {
        if index == 255 {
            255
        } else {
            indices[usize::from(index)]
        }
    });
    let info = image.info();
    let mut disk = Disk::new(map, tracks, info.optimal_bit_timing, info.write_protected)?;
    disk.set_writable(writable);
    Ok(disk)
}
/// Copy edited tracks back into a clone of their source container. Refuse a
/// different track/map shape. The caller exports these bytes to a new path.
pub fn export(image: &Woz, disk: &Disk) -> Result<Vec<u8>, DiskError> {
    let original = mount(image, false)?;
    if original.track_map() != disk.track_map() || original.tracks().len() != disk.tracks().len() {
        return Err(DiskError::Map);
    }
    let mut updated = image.clone();
    let mut dense = 0;
    for index in 0..160 {
        if let Some((_, count)) = image.track(index) {
            let track = &disk.tracks()[dense];
            if count != track.bit_count() {
                return Err(DiskError::Track);
            }
            updated
                .track_mut(index)
                .expect("validated source track")
                .copy_from_slice(track.bytes());
            dense += 1;
        }
    }
    Ok(updated.to_bytes())
}

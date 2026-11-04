"""Create an authored one-sector bootstrap; convert with dsk2woz2 externally."""
from pathlib import Path
import sys

# Disk II loads sector zero to $0800, then jumps to $0801. Print through the
# caller's Monitor COUT routine, leave an observable marker, and loop locally.
code = bytearray([1])
for char in b"DISK II BOOT OK":
    code.extend([0xA9, char | 0x80, 0x20, 0xED, 0xFD])
code.extend([0xA9, 0x42, 0x8D, 0x00, 0x03])
end = 0x800 + len(code)
code.extend([0x4C, end & 255, end >> 8])
image = code + bytes(143360 - len(code))
with Path(sys.argv[1]).open("xb") as output:
    output.write(image)

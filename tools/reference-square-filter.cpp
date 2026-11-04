// Reference fixture generator; not a build dependency.
// MAME apple2video.cpp, revision 9391e1eff80eb04abd58d48218a1d3b9643c1ea8.
// license:BSD-3-Clause; copyright-holders:R. Belmont.
// See THIRD_PARTY_NOTICES.md. Rotation/window code below is copied verbatim.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
static constexpr unsigned rotl4b(unsigned n, unsigned count) { return (n >> (-count & 3)) & 0x0f; }
static constexpr unsigned rotl4(unsigned n, unsigned count) { return rotl4b(n * 0x11, count); }

template <int ContextBits, typename F>
static void render_line_pixel_window(uint16_t *out, uint16_t const *in, int startcol, int stopcol, F &&color_function)
{
	static_assert(ContextBits >= 0 && ContextBits <= 4);  // uint32_t has room for 4+14+14 bits

	if (startcol >= stopcol)
		return;  // to avoid OOB read

	// w holds ContextBits bits of the previous 14-pixel group and the current and next groups.
	uint32_t w = (ContextBits && startcol > 0) ? (in[startcol - 1] >> (14 - ContextBits)) : 0;
	w += in[startcol] << ContextBits;

	for (int col = startcol; col < stopcol; col++)
	{
		if (col + 1 < 40)
			w += in[col + 1] << (14 + ContextBits);

		for (int b = 0; b < 14; b++)
		{
			out[col * 14 + b] = color_function(w, col * 14 + b);
			w >>= 1;
		}
	}
}


int main(int argc, char **argv) {
    if (argc != 3) return 2;
    FILE *table = std::fopen(argv[1], "w");
    FILE *row = std::fopen(argv[2], "w");
    if (!table || !row) return 3;
    std::fprintf(table, "# MAME composite mode 2: window,phase,index\n");
    for (unsigned window = 0; window < 16; ++window)
        for (unsigned phase = 0; phase < 4; ++phase)
            std::fprintf(table, "%u,%u,%u\n", window, phase, rotl4(window, phase - 1));
    const uint16_t inputs[] = {0x0003, 0x3000, 0x0003, 0x1555, 0x2aaa, 0x3333, 0, 0x3fff};
    uint16_t words[40], output[560];
    for (unsigned i = 0; i < 40; ++i) words[i] = inputs[i % 8];
    render_line_pixel_window<1>(output, words, 0, 40,
        [](uint32_t w, int x) { return rotl4(w & 0x0f, x - 1); });
    std::fprintf(row, "# MAME composite mode 2: x,index; words repeat 0003 3000 0003 1555 2aaa 3333 0000 3fff\n");
    for (unsigned x = 0; x < 560; ++x) std::fprintf(row, "%u,%u\n", x, output[x]);
    const int table_status = std::fclose(table);
    const int row_status = std::fclose(row);
    if (table_status || row_status) return 4;
    return 0;
}

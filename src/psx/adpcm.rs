pub const ADPCM_FILTERS: [[i16; 2]; 16] = [
    [0, 0],
    [60, 0],
    [115, 52],
    [98, 55],
    [122, 60],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
];

pub const ADPCM_ZIGZAG_TABLE: [[i32; 29]; 7] = [
    [0, 0, 0, 0, 0, -0x2, 0xa, -0x22, 0x41, -0x54, 0x34, 0x9, -0x10a, 0x400, -0xa78, 0x234c, 0x6794, -0x1780, 0xbcd, -0x623, 0x350, -0x16d, 0x6b, 0xa, -0x10, 0x11, -0x8, 0x3, -0x1],
    [0, 0, 0, -0x2, 0, 0x3, -0x13, 0x3c, -0x4b, 0xa2, -0xe3, 0x132, -0x43, -0x267, 0xc9d, 0x74bb, -0x11b4, 0x9b8, -0x5bf, 0x372, -0x1a8, 0xa6, -0x1b, 0x5, 0x6, -0x8, 0x3, -0x1, 0],
    [0, 0, -0x1, 0x3, -0x2, -0x5, 0x1f, -0x4a, 0xb3, -0x192, 0x2b1, -0x39e, 0x4f8, -0x5a6, 0x7939, -0x5a6, 0x4f8, -0x39e, 0x2b1, -0x192, 0xb3, -0x4a, 0x1f, -0x5, -0x2, 0x3, -0x1, 0, 0],
    [0, -0x1, 0x3, -0x8, 0x6, 0x5, -0x1b, 0xa6, -0x1a8, 0x372, -0x5bf, 0x9b8, -0x11b4, 0x74bb, 0xc9d, -0x267, -0x43, 0x132, -0xe3, 0xa2, -0x4b, 0x3c, -0x13, 0x3, 0, -0x2, 0, 0, 0],
    [-0x1, 0x3, -0x8, 0x11, -0x10, 0xa, 0x6b, -0x16d, 0x350, -0x623, 0xbcd, -0x1780, 0x6794, 0x234c, -0xa78, 0x400, -0x10a, 0x9, 0x34, -0x54, 0x41, -0x22, 0xa, -0x1, 0, 0x1, 0, 0, 0],
    [0x2, -0x8, 0x10, -0x23, 0x2b, 0x1a, -0xeb, 0x27b, -0x548, 0xafa, -0x16fa, 0x53e0, 0x3c07, -0x1249, 0x80e, -0x347, 0x15b, -0x44, -0x17, 0x46, -0x23, 0x11, -0x5, 0, 0, 0, 0, 0, 0],
    [-0x5, 0x11, -0x23, 0x46, -0x17, -0x44, 0x15b, -0x347, 0x80e, -0x1249, 0x3c07, 0x53e0, -0x16fa, 0xafa, -0x548, 0x27b, -0xeb, 0x1a, 0x2b, -0x23, 0x10, -0x8, 0x2, 0, 0, 0, 0, 0, 0],
];
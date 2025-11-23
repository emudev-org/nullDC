/**
 * SGC Channel Data Helper
 *
 * Provides typed access to the 128-byte ChannelCommonData structure
 * for each SGC audio channel.
 *
 * Structure layout (72 bytes of data + 56 bytes padding):
 * - Registers: 18 x 32-bit words (72 bytes)
 * - Sample data: 9 x 16-bit values (18 bytes)
 * - Additional state: various sized fields (20 bytes)
 * - Padding: remaining bytes zeroed
 */

export class SgcChannelData {
  private view: DataView;
  private offset: number;

  constructor(buffer: ArrayBuffer, channelIndex: number) {
    this.view = new DataView(buffer);
    this.offset = channelIndex * 128; // 128 bytes per channel
  }

  // Helper to read 32-bit word
  private readWord(wordIndex: number): number {
    return this.view.getUint32(this.offset + wordIndex * 4, true); // little-endian
  }

  // Helper to write 32-bit word
  private writeWord(wordIndex: number, value: number): void {
    this.view.setUint32(this.offset + wordIndex * 4, value, true);
  }

  // Helper to extract bits from a word
  private getBits(wordIndex: number, bitOffset: number, bitCount: number): number {
    const word = this.readWord(wordIndex);
    const mask = (1 << bitCount) - 1;
    return (word >> bitOffset) & mask;
  }

  // Helper to set bits in a word
  private setBits(wordIndex: number, bitOffset: number, bitCount: number, value: number): void {
    const word = this.readWord(wordIndex);
    const mask = (1 << bitCount) - 1;
    const cleared = word & ~(mask << bitOffset);
    const newWord = cleared | ((value & mask) << bitOffset);
    this.writeWord(wordIndex, newWord);
  }

  // +00 [0] - Sample Address High, PCMS, Loop Control, Sound Start Control, Key On
  get SA_hi(): number { return this.getBits(0, 0, 7); }
  set SA_hi(value: number) { this.setBits(0, 0, 7, value); }

  get PCMS(): number { return this.getBits(0, 7, 2); }
  set PCMS(value: number) { this.setBits(0, 7, 2, value); }

  get LPCTL(): number { return this.getBits(0, 9, 1); }
  set LPCTL(value: number) { this.setBits(0, 9, 1, value); }

  get SSCTL(): number { return this.getBits(0, 10, 1); }
  set SSCTL(value: number) { this.setBits(0, 10, 1, value); }

  get KYONB(): number { return this.getBits(0, 14, 1); }
  set KYONB(value: number) { this.setBits(0, 14, 1, value); }

  get KYONEX(): number { return this.getBits(0, 15, 1); }
  set KYONEX(value: number) { this.setBits(0, 15, 1, value); }

  // +04 [1] - Sample Address Low
  get SA_low(): number { return this.getBits(1, 0, 16); }
  set SA_low(value: number) { this.setBits(1, 0, 16, value); }

  // Full Sample Address (combined SA_hi and SA_low)
  get SA(): number { return (this.SA_hi << 16) | this.SA_low; }
  set SA(value: number) {
    this.SA_hi = (value >> 16) & 0x7F;
    this.SA_low = value & 0xFFFF;
  }

  // +08 [2] - Loop Start Address
  get LSA(): number { return this.getBits(2, 0, 16); }
  set LSA(value: number) { this.setBits(2, 0, 16, value); }

  // +0C [3] - Loop End Address
  get LEA(): number { return this.getBits(3, 0, 16); }
  set LEA(value: number) { this.setBits(3, 0, 16, value); }

  // +10 [4] - Attack Rate, Decay 1 Rate, Decay 2 Rate
  get AR(): number { return this.getBits(4, 0, 5); }
  set AR(value: number) { this.setBits(4, 0, 5, value); }

  get D1R(): number { return this.getBits(4, 6, 5); }
  set D1R(value: number) { this.setBits(4, 6, 5, value); }

  get D2R(): number { return this.getBits(4, 11, 5); }
  set D2R(value: number) { this.setBits(4, 11, 5, value); }

  // +14 [5] - Release Rate, Decay Level, Key Rate Scaling, Loop Start Link
  get RR(): number { return this.getBits(5, 0, 5); }
  set RR(value: number) { this.setBits(5, 0, 5, value); }

  get DL(): number { return this.getBits(5, 5, 5); }
  set DL(value: number) { this.setBits(5, 5, 5, value); }

  get KRS(): number { return this.getBits(5, 10, 4); }
  set KRS(value: number) { this.setBits(5, 10, 4, value); }

  get LPSLNK(): number { return this.getBits(5, 14, 1); }
  set LPSLNK(value: number) { this.setBits(5, 14, 1, value); }

  // +18 [6] - Frequency Number Step, Octave
  get FNS(): number { return this.getBits(6, 0, 10); }
  set FNS(value: number) { this.setBits(6, 0, 10, value); }

  get OCT(): number { return this.getBits(6, 11, 4); }
  set OCT(value: number) { this.setBits(6, 11, 4, value); }

  // +1C [7] - LFO parameters
  get ALFOS(): number { return this.getBits(7, 0, 3); }
  set ALFOS(value: number) { this.setBits(7, 0, 3, value); }

  get ALFOWS(): number { return this.getBits(7, 3, 2); }
  set ALFOWS(value: number) { this.setBits(7, 3, 2, value); }

  get PLFOS(): number { return this.getBits(7, 5, 3); }
  set PLFOS(value: number) { this.setBits(7, 5, 3, value); }

  get PLFOWS(): number { return this.getBits(7, 8, 2); }
  set PLFOWS(value: number) { this.setBits(7, 8, 2, value); }

  get LFOF(): number { return this.getBits(7, 10, 5); }
  set LFOF(value: number) { this.setBits(7, 10, 5, value); }

  get LFORE(): number { return this.getBits(7, 15, 1); }
  set LFORE(value: number) { this.setBits(7, 15, 1, value); }

  // +20 [8] - Input Select, Input Mix Level
  get ISEL(): number { return this.getBits(8, 0, 4); }
  set ISEL(value: number) { this.setBits(8, 0, 4, value); }

  get IMXL(): number { return this.getBits(8, 4, 4); }
  set IMXL(value: number) { this.setBits(8, 4, 4, value); }

  // +24 [9] - Direct Pan, Direct Send Level
  get DIPAN(): number { return this.getBits(9, 0, 5); }
  set DIPAN(value: number) { this.setBits(9, 0, 5, value); }

  get DISDL(): number { return this.getBits(9, 8, 4); }
  set DISDL(value: number) { this.setBits(9, 8, 4, value); }

  // +28 [10] - Q (Filter Resonance), Total Level
  get Q(): number { return this.getBits(10, 0, 5); }
  set Q(value: number) { this.setBits(10, 0, 5, value); }

  get TL(): number { return this.getBits(10, 8, 8); }
  set TL(value: number) { this.setBits(10, 8, 8, value); }

  // +2C [11] - Filter Level 0
  get FLV0(): number { return this.getBits(11, 0, 13); }
  set FLV0(value: number) { this.setBits(11, 0, 13, value); }

  // +30 [12] - Filter Level 1
  get FLV1(): number { return this.getBits(12, 0, 13); }
  set FLV1(value: number) { this.setBits(12, 0, 13, value); }

  // +34 [13] - Filter Level 2
  get FLV2(): number { return this.getBits(13, 0, 13); }
  set FLV2(value: number) { this.setBits(13, 0, 13, value); }

  // +38 [14] - Filter Level 3
  get FLV3(): number { return this.getBits(14, 0, 13); }
  set FLV3(value: number) { this.setBits(14, 0, 13, value); }

  // +3C [15] - Filter Level 4
  get FLV4(): number { return this.getBits(15, 0, 13); }
  set FLV4(value: number) { this.setBits(15, 0, 13, value); }

  // +40 [16] - Filter Decay 1 Rate, Filter Attack Rate
  get FD1R(): number { return this.getBits(16, 0, 5); }
  set FD1R(value: number) { this.setBits(16, 0, 5, value); }

  get FAR(): number { return this.getBits(16, 8, 5); }
  set FAR(value: number) { this.setBits(16, 8, 5, value); }

  // +44 [17] - Filter Release Rate, Filter Decay 2 Rate
  get FRR(): number { return this.getBits(17, 0, 5); }
  set FRR(value: number) { this.setBits(17, 0, 5, value); }

  get FD2R(): number { return this.getBits(17, 8, 5); }
  set FD2R(value: number) { this.setBits(17, 8, 5, value); }

  // Sample data (starting at byte 72, after the 18 registers)
  private getSample(index: number): number {
    return this.view.getInt16(this.offset + 72 + index * 2, true);
  }

  private setSample(index: number, value: number): void {
    this.view.setInt16(this.offset + 72 + index * 2, value, true);
  }

  // +48 [72-73] - Current sample value
  get sample_current(): number { return this.getSample(0); }
  set sample_current(value: number) { this.setSample(0, value); }

  // +4A [74-75] - Previous sample value
  get sample_previous(): number { return this.getSample(1); }
  set sample_previous(value: number) { this.setSample(1, value); }

  // +4C [76-77] - Filtered sample
  get sample_filtered(): number { return this.getSample(2); }
  set sample_filtered(value: number) { this.setSample(2, value); }

  // +4E [78-79] - Sample after AEG (Amplitude Envelope Generator)
  get sample_post_aeg(): number { return this.getSample(3); }
  set sample_post_aeg(value: number) { this.setSample(3, value); }

  // +50 [80-81] - Sample after FEG (Filter Envelope Generator)
  get sample_post_feg(): number { return this.getSample(4); }
  set sample_post_feg(value: number) { this.setSample(4, value); }

  // +52 [82-83] - Sample after Total Level
  get sample_post_tl(): number { return this.getSample(5); }
  set sample_post_tl(value: number) { this.setSample(5, value); }

  // +54 [84-85] - Left channel output
  get sample_left(): number { return this.getSample(6); }
  set sample_left(value: number) { this.setSample(6, value); }

  // +56 [86-87] - Right channel output
  get sample_right(): number { return this.getSample(7); }
  set sample_right(value: number) { this.setSample(7, value); }

  // +58 [88-89] - DSP send
  get sample_dsp(): number { return this.getSample(8); }
  set sample_dsp(value: number) { this.setSample(8, value); }

  // Additional state fields (starting at byte 90)

  // +5A [90-91] - CA (Current Address) fraction (10 bits) + padding
  get ca_fraction(): number {
    const word = this.view.getUint16(this.offset + 90, true);
    return word & 0x3FF; // 10 bits
  }
  set ca_fraction(value: number) {
    const word = this.view.getUint16(this.offset + 90, true);
    const newWord = (word & ~0x3FF) | (value & 0x3FF);
    this.view.setUint16(this.offset + 90, newWord, true);
  }

  // +5C [92-95] - CA step (32-bit)
  get ca_step(): number {
    return this.view.getUint32(this.offset + 92, true);
  }
  set ca_step(value: number) {
    this.view.setUint32(this.offset + 92, value, true);
  }

  // +60 [96-99] - AEG value (32-bit)
  get aeg_value(): number {
    return this.view.getUint32(this.offset + 96, true);
  }
  set aeg_value(value: number) {
    this.view.setUint32(this.offset + 96, value, true);
  }

  // +64 [100-103] - FEG value (32-bit)
  get feg_value(): number {
    return this.view.getUint32(this.offset + 100, true);
  }
  set feg_value(value: number) {
    this.view.setUint32(this.offset + 100, value, true);
  }

  // +68 [104] - LFO value (8-bit)
  get lfo_value(): number {
    return this.view.getUint8(this.offset + 104);
  }
  set lfo_value(value: number) {
    this.view.setUint8(this.offset + 104, value);
  }

  // +69 [105] - Amplitude LFO value (8-bit)
  get alfo_value(): number {
    return this.view.getUint8(this.offset + 105);
  }
  set alfo_value(value: number) {
    this.view.setUint8(this.offset + 105, value);
  }

  // +6A [106] - Pitch LFO value (8-bit)
  get plfo_value(): number {
    return this.view.getUint8(this.offset + 106);
  }
  set plfo_value(value: number) {
    this.view.setUint8(this.offset + 106, value);
  }

  // +6B [107-108] - CA (Current Address) current (16-bit)
  get ca_current(): number {
    return this.view.getUint16(this.offset + 107, true);
  }
  set ca_current(value: number) {
    this.view.setUint16(this.offset + 107, value, true);
  }

  // Remaining bytes (109-127) are padding/reserved

  /**
   * Get all channel data as a plain object
   */
  toObject(): Record<string, number> {
    return {
      // Register data
      SA_hi: this.SA_hi,
      SA_low: this.SA_low,
      SA: this.SA,
      PCMS: this.PCMS,
      LPCTL: this.LPCTL,
      SSCTL: this.SSCTL,
      KYONB: this.KYONB,
      KYONEX: this.KYONEX,
      LSA: this.LSA,
      LEA: this.LEA,
      AR: this.AR,
      D1R: this.D1R,
      D2R: this.D2R,
      RR: this.RR,
      DL: this.DL,
      KRS: this.KRS,
      LPSLNK: this.LPSLNK,
      FNS: this.FNS,
      OCT: this.OCT,
      ALFOS: this.ALFOS,
      ALFOWS: this.ALFOWS,
      PLFOS: this.PLFOS,
      PLFOWS: this.PLFOWS,
      LFOF: this.LFOF,
      LFORE: this.LFORE,
      ISEL: this.ISEL,
      IMXL: this.IMXL,
      DIPAN: this.DIPAN,
      DISDL: this.DISDL,
      Q: this.Q,
      TL: this.TL,
      FLV0: this.FLV0,
      FLV1: this.FLV1,
      FLV2: this.FLV2,
      FLV3: this.FLV3,
      FLV4: this.FLV4,
      FD1R: this.FD1R,
      FAR: this.FAR,
      FRR: this.FRR,
      FD2R: this.FD2R,
      // Sample data
      sample_current: this.sample_current,
      sample_previous: this.sample_previous,
      sample_filtered: this.sample_filtered,
      sample_post_aeg: this.sample_post_aeg,
      sample_post_feg: this.sample_post_feg,
      sample_post_tl: this.sample_post_tl,
      sample_left: this.sample_left,
      sample_right: this.sample_right,
      sample_dsp: this.sample_dsp,
      // Additional state
      ca_fraction: this.ca_fraction,
      ca_step: this.ca_step,
      ca_current: this.ca_current,
      aeg_value: this.aeg_value,
      feg_value: this.feg_value,
      lfo_value: this.lfo_value,
      alfo_value: this.alfo_value,
      plfo_value: this.plfo_value,
    };
  }
}

/**
 * Helper to access all channels in a frame
 */
export class SgcFrameData {
  private buffer: ArrayBuffer;
  private channels: SgcChannelData[] = [];

  constructor(buffer: ArrayBuffer, frameIndex: number = 0) {
    const BYTES_PER_FRAME = 8192; // 64 channels × 128 bytes
    const frameOffset = frameIndex * BYTES_PER_FRAME;

    // Create a view into the specific frame
    this.buffer = buffer.slice(frameOffset, frameOffset + BYTES_PER_FRAME);

    // Initialize channel accessors
    for (let i = 0; i < 64; i++) {
      this.channels[i] = new SgcChannelData(this.buffer, i);
    }
  }

  /**
   * Get channel data accessor for a specific channel (0-63)
   */
  getChannel(channelIndex: number): SgcChannelData {
    if (channelIndex < 0 || channelIndex >= 64) {
      throw new Error(`Channel index ${channelIndex} out of range (0-63)`);
    }
    return this.channels[channelIndex];
  }

  /**
   * Get the underlying ArrayBuffer for this frame
   */
  getBuffer(): ArrayBuffer {
    return this.buffer;
  }
}

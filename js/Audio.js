// This code is intentionally left unminified for your perusal
(function() {
class SquareChannel {
  constructor(ctx) {
    this.ctx = ctx;
    this.oscillator = ctx.createOscillator();
    this.gain = ctx.createGain();
    this.oscillator.connect(this.gain);
    this.oscillator.type = 'square';
    this.oscillator.frequency.setValueAtTime(0, ctx.currentTime);
    this.gain.gain.setValueAtTime(1, ctx.currentTime);
    this.oscillator.start(0);
  }

  connect(node) {
    this.gain.connect(node);
  }

  setGain(g) {
    this.gain.gain.setValueAtTime(Math.max(g, 0.00001), this.ctx.currentTime);
  }

  setFrequency(f) {
    this.oscillator.frequency.setValueAtTime(f, this.ctx.currentTime);
  }
}

class NoiseChannel {
  constructor(ctx) {
    this.ctx = ctx;
    const size = ctx.sampleRate; // 1s
    const buffer = ctx.createBuffer(1, size, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < size; i++) {
      data[i] = 2 * Math.random() - 1;
    }

    this.source = ctx.createBufferSource();
    this.source.buffer = buffer;
    this.source.loop = true;
    this.gain = ctx.createGain();
    this.source.connect(this.gain);
    this.gain.gain.setValueAtTime(0.00001, ctx.currentTime);
    this.source.start(0);
  }

  connect(node) {
    this.gain.connect(node);
  }

  setGain(g) {
    this.gain.gain.setValueAtTime(Math.max(g, 0.00001), this.ctx.currentTime);
  }
}

class Audio {
  constructor() {
    this.ctx = new AudioContext();
    this.ctx.suspend();

    this.channels = [
      new SquareChannel(this.ctx),
      new SquareChannel(this.ctx),
      null,
      new NoiseChannel(this.ctx),
    ];

    this.leftControls = [
      this.ctx.createGain(),
      this.ctx.createGain(),
      this.ctx.createGain(),
      this.ctx.createGain(),
    ];
    this.rightControls = [
      this.ctx.createGain(),
      this.ctx.createGain(),
      this.ctx.createGain(),
      this.ctx.createGain(),
    ];

    this.masterLeftGain = this.ctx.createGain();
    this.masterRightGain = this.ctx.createGain();
    const merger = this.ctx.createChannelMerger();
    this.masterLeftGain.connect(merger, 0, 0);
    this.masterRightGain.connect(merger, 0, 1);
    this.master = this.ctx.createGain();
    merger.connect(this.master);
    this.master.connect(this.ctx.destination);

    for (let i = 0; i < 4; i++) {
      if (this.channels[i]) {
        this.channels[i].connect(this.leftControls[i], 0);
        this.channels[i].connect(this.rightControls[i], 1);
        this.leftControls[i].connect(this.masterLeftGain);
        this.rightControls[i].connect(this.masterRightGain);
      }
    }
  }

  setMasterGain(left, right) {
    this.masterLeftGain.gain.setValueAtTime(Math.max(left, 0.00001), this.ctx.currentTime);
    this.masterRightGain.gain.setValueAtTime(Math.max(right, 0.00001), this.ctx.currentTime);
  }

  pause() {
    this.ctx.suspend();
  }

  play() {
    this.ctx.resume();
  }

  enableAudio(flag) {
    if (flag) {
      this.master.gain.setValueAtTime(1.0, this.ctx.currentTime);
    } else {
      this.master.gain.setValueAtTime(0.00001, this.ctx.currentTime);
    }
  }
}

window.Audio = Audio;
})();
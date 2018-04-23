// This code is intentionally left unminified for your perusal
(function() {

class Controls {
  constructor() {
    this._buttonLast = 0xf;
    this._directionLast = 0xf;

    this._buttonNext = 0xf;
    this._directionNext = 0xf;

    this._controller = -1;
    this._controllerButtonMapping = [0, 1, 2, 3, 15, 14, 12, 13]; // A, B, Select, Start, [Right, Left, Up, Down]
    this._controllerAxisMapping = [0, 1]; // LR, UD

    document.addEventListener('keydown', e => {
      switch (e.keyCode) {
        case 88:
          this._buttonNext &= 0xe; // A
          break;
        case 90:
          this._buttonNext &= 0xd; // B
          break;
        case 16:
          this._buttonNext &= 0xb; // Select
          break;
        case 13:
          this._buttonNext &= 0x7; // Start
          break;
        case 39:
          this._directionNext &= 0xe; // Right
          break;
        case 37:
          this._directionNext &= 0xd; // Left
          break;
        case 38:
          this._directionNext &= 0xb; // Up
          break;
        case 40:
          this._directionNext &= 0x7; // Down
          break;
        default:
          return;
      }
      e.preventDefault();
    });

    document.addEventListener('keyup', e => {
      switch (e.keyCode) {
        case 88:
          this._buttonNext |= 0x1; // A
          break;
        case 90:
          this._buttonNext |= 0x2; // B
          break;
        case 16:
          this._buttonNext |= 0x4; // Select
          break;
        case 13:
          this._buttonNext |= 0x8; // Start
          break;
        case 39:
          this._directionNext |= 0x1; // Right
          break;
        case 37:
          this._directionNext |= 0x2; // Left
          break;
        case 38:
          this._directionNext |= 0x4; // Up
          break;
        case 40:
          this._directionNext |= 0x8; // Down
          break;
        default:
          return;
      }
      e.preventDefault();
    });

    window.addEventListener('gamepadconnected', e => {
      if (this._controller > -1) {
        // We already have a captive controller
        return;
      }
      const gamepad = e.gamepad;
      if (gamepad.buttons < 4 || gamepad.axes < 2) {
        // Can't play GB games
        return;
      }
      this._controller = gamepad.index;
    });

    window.addEventListener('gamepaddisconnected', e => {
      if (this._controller < 0) {
        return;
      }
      if (e.gamepad.index === this._controller) {
        this._controller = -1;
      }
    });

    this.onscreen = {
      a: document.getElementById('control_a'),
      b: document.getElementById('control_b'),
      start: document.getElementById('control_start'),
      select: document.getElementById('control_select'),
      right: document.getElementById('control_right'),
      left: document.getElementById('control_left'),
      up: document.getElementById('control_up'),
      down: document.getElementById('control_down'),
    };
    this.onscreen.a.addEventListener('touchstart', this.touchStartButton.bind(this, 0xe));
    this.onscreen.b.addEventListener('touchstart', this.touchStartButton.bind(this, 0xd));
    this.onscreen.select.addEventListener('touchstart', this.touchStartButton.bind(this, 0xb));
    this.onscreen.start.addEventListener('touchstart', this.touchStartButton.bind(this, 0x7));
    this.onscreen.right.addEventListener('touchstart', this.touchStartDirection.bind(this, 0xe));
    this.onscreen.left.addEventListener('touchstart', this.touchStartDirection.bind(this, 0xd));
    this.onscreen.up.addEventListener('touchstart', this.touchStartDirection.bind(this, 0xb));
    this.onscreen.down.addEventListener('touchstart', this.touchStartDirection.bind(this, 0x7));

    this.onscreen.right.addEventListener('touchend', this.touchStopDirection.bind(this, 0x1));
    this.onscreen.left.addEventListener('touchend', this.touchStopDirection.bind(this, 0x2));
    this.onscreen.up.addEventListener('touchend', this.touchStopDirection.bind(this, 0x4));
    this.onscreen.down.addEventListener('touchend', this.touchStopDirection.bind(this, 0x8));
    this.onscreen.a.addEventListener('touchend', this.touchStopButton.bind(this, 0x1));
    this.onscreen.b.addEventListener('touchend', this.touchStopButton.bind(this, 0x2));
    this.onscreen.select.addEventListener('touchend', this.touchStopButton.bind(this, 0x4));
    this.onscreen.start.addEventListener('touchend', this.touchStopButton.bind(this, 0x8));

    this.onscreen.right.addEventListener('touchcancel', this.touchStopDirection.bind(this, 0x1));
    this.onscreen.left.addEventListener('touchcancel', this.touchStopDirection.bind(this, 0x2));
    this.onscreen.up.addEventListener('touchcancel', this.touchStopDirection.bind(this, 0x4));
    this.onscreen.down.addEventListener('touchcancel', this.touchStopDirection.bind(this, 0x8));
    this.onscreen.a.addEventListener('touchcancel', this.touchStopButton.bind(this, 0x1));
    this.onscreen.b.addEventListener('touchcancel', this.touchStopButton.bind(this, 0x2));
    this.onscreen.select.addEventListener('touchcancel', this.touchStopButton.bind(this, 0x4));
    this.onscreen.start.addEventListener('touchcancel', this.touchStopButton.bind(this, 0x8));
  }

  touchStartButton(mask, e) {
    this._buttonNext &= mask;
    e.preventDefault();
  }

  touchStopButton(mask, e) {
    this._buttonNext |= mask;
    e.preventDefault();
  }

  touchStartDirection(mask, e) {
    this._directionNext &= mask;
    e.preventDefault();
  }

  touchStopDirection(mask, e) {
    this._directionNext |= mask;
    e.preventDefault();
  }

  pollControllerButtons() {
    const gamepads = navigator.getGamepads();
    const gamepad = gamepads[this._controller];
    if (!gamepad) {
      return;
    }
    let buttons = 0xf;
    const mapping = this._controllerButtonMapping;
    if (gamepad.buttons[mapping[0]].pressed) {
      buttons &= 0xe;
    }
    if (gamepad.buttons[mapping[1]].pressed) {
      buttons &= 0xd;
    }
    if (gamepad.buttons[mapping[2]].pressed) {
      buttons &= 0xb;
    }
    if (gamepad.buttons[mapping[3]].pressed) {
      buttons &= 0x7;
    }
    return buttons;
  }

  pollControllerDirections() {
    const gamepads = navigator.getGamepads();
    const gamepad = gamepads[this._controller];
    if (!gamepad) {
      return;
    }
    let directions = 0xf;
    const mapping = this._controllerAxisMapping;
    const buttonMapping = this._controllerButtonMapping;
    if (buttonMapping.length > 4) {
      if (gamepad.buttons[buttonMapping[4]] && gamepad.buttons[buttonMapping[4]].pressed) {
        directions &= 0xe;
      }
      if (gamepad.buttons[buttonMapping[5]] && gamepad.buttons[buttonMapping[5]].pressed) {
        directions &= 0xd;
      }
      if (gamepad.buttons[buttonMapping[6]] && gamepad.buttons[buttonMapping[6]].pressed) {
        directions &= 0xb;
      }
      if (gamepad.buttons[buttonMapping[7]] && gamepad.buttons[buttonMapping[7]].pressed) {
        directions &= 0x7;
      }
    }
    if (gamepad.axes[mapping[0]] < -0.3) {
      directions &= 0xd;
    }
    if (gamepad.axes[mapping[0]] > 0.3) {
      directions &= 0xe;
    }
    if (gamepad.axes[mapping[1]] < -0.3) {
      directions &= 0xb;
    }
    if (gamepad.axes[mapping[1]] > 0.3) {
      directions &= 0x7;
    }
    return directions;
  }

  getButtons() {
    let next = this._buttonNext;
    if (this._controller > -1) {
      const gamepadButtons = this.pollControllerButtons();
      next &= gamepadButtons;
    }
    if (next === this._buttonLast) {
      return null; // No change
    }
    this._buttonLast = next;
    return next;
  }

  getDirections() {
    let next = this._directionNext;
    if (this._controller > -1) {
      const gamepadDirections = this.pollControllerDirections();
      next &= gamepadDirections;
    }
    if (next === this._directionLast) {
      return null; // No change
    }
    this._directionLast = next;
    return next;
  }
}

window.Controls = Controls;

})();
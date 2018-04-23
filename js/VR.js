(function() {

const vertexShaderSource = `#version 300 es
in vec4 a_position;
in vec2 a_texcoord;

uniform mat4 u_matrix;

out vec2 v_texcoord;

void main() {
  gl_Position = u_matrix * a_position;
  v_texcoord = a_texcoord;
}
`;

const fragmentShaderSource = `#version 300 es
precision mediump float;

in vec2 v_texcoord;

uniform sampler2D u_texture;

out vec4 outColor;

void main() {
  outColor = texture(u_texture, v_texcoord);
}
`;

function multiply(a, b) {
  var a00 = a[0 * 4 + 0];
  var a01 = a[0 * 4 + 1];
  var a02 = a[0 * 4 + 2];
  var a03 = a[0 * 4 + 3];
  var a10 = a[1 * 4 + 0];
  var a11 = a[1 * 4 + 1];
  var a12 = a[1 * 4 + 2];
  var a13 = a[1 * 4 + 3];
  var a20 = a[2 * 4 + 0];
  var a21 = a[2 * 4 + 1];
  var a22 = a[2 * 4 + 2];
  var a23 = a[2 * 4 + 3];
  var a30 = a[3 * 4 + 0];
  var a31 = a[3 * 4 + 1];
  var a32 = a[3 * 4 + 2];
  var a33 = a[3 * 4 + 3];
  var b00 = b[0 * 4 + 0];
  var b01 = b[0 * 4 + 1];
  var b02 = b[0 * 4 + 2];
  var b03 = b[0 * 4 + 3];
  var b10 = b[1 * 4 + 0];
  var b11 = b[1 * 4 + 1];
  var b12 = b[1 * 4 + 2];
  var b13 = b[1 * 4 + 3];
  var b20 = b[2 * 4 + 0];
  var b21 = b[2 * 4 + 1];
  var b22 = b[2 * 4 + 2];
  var b23 = b[2 * 4 + 3];
  var b30 = b[3 * 4 + 0];
  var b31 = b[3 * 4 + 1];
  var b32 = b[3 * 4 + 2];
  var b33 = b[3 * 4 + 3];
  return [
    b00 * a00 + b01 * a10 + b02 * a20 + b03 * a30,
    b00 * a01 + b01 * a11 + b02 * a21 + b03 * a31,
    b00 * a02 + b01 * a12 + b02 * a22 + b03 * a32,
    b00 * a03 + b01 * a13 + b02 * a23 + b03 * a33,
    b10 * a00 + b11 * a10 + b12 * a20 + b13 * a30,
    b10 * a01 + b11 * a11 + b12 * a21 + b13 * a31,
    b10 * a02 + b11 * a12 + b12 * a22 + b13 * a32,
    b10 * a03 + b11 * a13 + b12 * a23 + b13 * a33,
    b20 * a00 + b21 * a10 + b22 * a20 + b23 * a30,
    b20 * a01 + b21 * a11 + b22 * a21 + b23 * a31,
    b20 * a02 + b21 * a12 + b22 * a22 + b23 * a32,
    b20 * a03 + b21 * a13 + b22 * a23 + b23 * a33,
    b30 * a00 + b31 * a10 + b32 * a20 + b33 * a30,
    b30 * a01 + b31 * a11 + b32 * a21 + b33 * a31,
    b30 * a02 + b31 * a12 + b32 * a22 + b33 * a32,
    b30 * a03 + b31 * a13 + b32 * a23 + b33 * a33,
  ];
}

class VRState {
  constructor() {
    this.display = null;

    this.onDisplayConnect = this.onDisplayConnect.bind(this);
    this.onDisplayDisconnect = this.onDisplayDisconnect.bind(this);
    this.onDisplayPresentChange = this.onDisplayPresentChange.bind(this);
    this._enterCallbacks = [];
    this._exitCallbacks = [];
    this._displayChangeCallbacks = [];

    window.addEventListener('vrdisplayconnect', this.onDisplayConnect);
    window.addEventListener('vrdisplaydisconnect', this.onDisplayDisconnect);
    window.addEventListener('vrdisplaypresentchange', this.onDisplayPresentChange);

    if (typeof navigator.getVRDisplays === 'function') {
      navigator.getVRDisplays().then(displays => {
        if (displays.length) {
          this.setCurrentDisplay(displays[0]);
        }
      }).catch(err => {
        console.error(err);
      });
    }
  }

  _callEnterCallbacks() {
    for (let i = 0; i < this._enterCallbacks.length; i++) {
      this._enterCallbacks[i]();
    }
  }

  _callExitCallbacks() {
    for (let i = 0; i < this._exitCallbacks.length; i++) {
      this._exitCallbacks[i]();
    }
  }

  _callDisplayChangeCallbacks() {
    for (let i = 0; i < this._displayChangeCallbacks.length; i++) {
      this._displayChangeCallbacks[i](this.display);
    }
  }

  setCurrentDisplay(display) {
    this.display = display;
    this._callDisplayChangeCallbacks();
  }

  isPresenting() {
    return !!this.display && this.display.isPresenting;
  }

  onEnter(cb) {
    this._enterCallbacks.push(cb);
  }

  onExit(cb) {
    this._exitCallbacks.push(cb);
  }

  onDisplayChange(cb) {
    this._displayChangeCallbacks.push(cb);
  }

  onDisplayConnect(e) {
    if (this.display) {
      return;
    }
    this.setCurrentDisplay(e.display);
  }

  onDisplayDisconnect(e) {
    const display = e.display;
    if (display !== this.display) {
      return;
    }
    this.setCurrentDisplay(null);
  }

  onDisplayPresentChange(e) {
    const display = e.display;
    if (this.display && display && this.display.displayId === display.displayId) {
      if (!display.isPresenting) {
        this._callExitCallbacks();
      }
    }
  }
}

class VR {
  constructor(gl) {
    this.gl = gl;
    this.active = false;

    this.state = new VRState();
    this.frameData = new VRFrameData();

    this.state.onEnter(() => {
      this.active = true;
    });
    this.state.onExit(() => {
      this.active = false;
    });

    this.program = Graphics.createProgram(
      gl,
      Graphics.createShader(gl, gl.VERTEX_SHADER, vertexShaderSource),
      Graphics.createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource),
    );
    this.positionAttributeLocation = gl.getAttribLocation(this.program, 'a_position');
    this.texcoordAttributeLocation = gl.getAttribLocation(this.program, 'a_texcoord');
    this.matrixLocation = gl.getUniformLocation(this.program, 'u_matrix');
    this.textureLocation = gl.getUniformLocation(this.program, 'u_texture');
    const positionBuffer = gl.createBuffer();
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    gl.enableVertexAttribArray(this.positionAttributeLocation);
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    // 1.60m x 1.44m screen
    const positions = new Float32Array([
      -0.80, 0.72, -2,
      -0.80, -0.72, -2,
      0.80, 0.72, -2,

      -0.80, -0.72, -2,
      0.80, -0.72, -2,
      0.80, 0.72, -2,
    ]);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
    gl.vertexAttribPointer(this.positionAttributeLocation, 3, gl.FLOAT, false, 0, 0);

    const texcoordBuffer = gl.createBuffer();
    gl.enableVertexAttribArray(this.texcoordAttributeLocation);
    gl.bindBuffer(gl.ARRAY_BUFFER, texcoordBuffer);
    const coords = new Float32Array([
      0, 1,
      0, 0,
      1, 1,
      0, 0,
      1, 0,
      1, 1,
    ]);
    gl.bufferData(gl.ARRAY_BUFFER, coords, gl.STATIC_DRAW);
    gl.vertexAttribPointer(this.texcoordAttributeLocation, 2, gl.FLOAT, false, 0, 0);

    this.texture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0 + 6);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 320, 288, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    this.fb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.texture, 0);
  }

  draw() {
    if (!this.active) {
      return;
    }
    const gl = this.gl;
    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(this.program);
    gl.bindVertexArray(this.vao);

    this.state.display.getFrameData(this.frameData);

    gl.uniform1i(this.textureLocation, 6);

    const leftViewMatrix = this.frameData.leftViewMatrix;
    const leftProjectionMatrix = this.frameData.leftProjectionMatrix;
    const leftMat = multiply(leftProjectionMatrix, leftViewMatrix);

    const rightViewMatrix = this.frameData.rightViewMatrix;
    const rightProjectionMatrix = this.frameData.rightProjectionMatrix;
    const rightMat = multiply(rightProjectionMatrix, rightViewMatrix);

    gl.enable(gl.SCISSOR_TEST);
    let x = 0;
    let y = 0;
    let w = 0.5 * gl.canvas.width;
    let h = gl.canvas.height;
    gl.viewport(x, y, w, h);
    gl.scissor(x, y, w, h);
    gl.uniformMatrix4fv(this.matrixLocation, false, leftMat);
    gl.drawArrays(gl.TRIANGLES, 0, 6);

    x = w;
    gl.viewport(x, y, w, h);
    gl.scissor(x, y, w, h);
    gl.uniformMatrix4fv(this.matrixLocation, false, rightMat);
    gl.drawArrays(gl.TRIANGLES, 0, 6);

    gl.disable(gl.SCISSOR_TEST);

    this.state.display.submitFrame();
  }
}

window.VR = VR;
})();
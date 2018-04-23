// This code is intentionally left unminified for your perusal
(function() {

const COLORS = [
  [0x9b, 0xbc, 0x0f],
  [0x8b, 0xac, 0x0f],
  [0x30, 0x62, 0x30],
  [0x0f, 0x38, 0x0f],
];

const GL_COLORS = [];
for (let i = 0; i < COLORS.length; i++) {
  GL_COLORS[i] = [];
  for (let j = 0; j < 3; j++) {
    GL_COLORS[i][j] = COLORS[i][j] / 255;
  }
}

const SPRITE_COUNT = 40;

const spriteGeom = new Float32Array([
  0, 0, 0, 8, 8, 0,
  8, 0, 0, 8, 8, 8,
]);

const tileVertexShaderSource = `#version 300 es
in vec4 a_position;
out vec2 v_texcoord;

uniform vec2 u_offset;

void main() {
  gl_Position = a_position * 2.0 - vec4(1, 1, 0, 1);
  v_texcoord = vec2(a_position.x * (160. / 256.) + u_offset.x, (1.0 - a_position.y) * (144. / 256.) + u_offset.y);
}
`;

const tileFragmentShaderSource = `#version 300 es
precision highp float;
precision highp usampler2D;

in vec2 v_texcoord;
uniform float u_atlaslength;
uniform usampler2D u_tile0texture;
uniform usampler2D u_tile1texture;
uniform usampler2D u_atlastexture;
uniform mat4 u_colors;
uniform int u_control;

out vec4 outColor;

void main() {
  if ((u_control & 0x80) > 0) {
    uint r0 = texture(u_tile0texture, v_texcoord).r;
    uint r1 = texture(u_tile1texture, v_texcoord).r;
    uint r = r0;
    if ((u_control & 0x8) > 0) {
      // BG uses higher tilemap
      r = r1;
    }
    if ((u_control & 0x10) == 0 && r < 128u) {
      r += 256u;
    }

    float tilex = v_texcoord.x * 8.0 * 32.0;
    float tiley = v_texcoord.y * 8.0 * 32.0;
    int offsetx = 7 - int(tilex - (floor(tilex / 8.0) * 8.0));
    float offsety = (tiley - (floor(tiley / 8.0) * 8.0)) / 8.0;
    vec2 atlascoord = vec2(offsety, (float(r) + 0.5) / u_atlaslength);
    uint high = texture(u_atlastexture, atlascoord).r;
    uint low = texture(u_atlastexture, atlascoord).g;

    uint hbit = (high >> offsetx) & 1u;
    uint lbit = (low >> offsetx) & 1u;
    uint color = hbit | (lbit << 1);

    outColor = u_colors[color];
  } else {
    outColor = u_colors[0];
  }
}
`;

const windowVertexShaderSource = `#version 300 es
precision highp float;

in vec4 a_position;
out vec2 v_texcoord;

uniform vec2 u_offset;

void main() {
  gl_Position = vec4(
    (a_position.x * 2. * (256. / 160.) - 1.) + ((u_offset.x - 7.) / 80.),
    (1. - a_position.y * 2. * (256. / 144.) - (u_offset.y / 72.)),
    0,
    1
  );
  v_texcoord = vec2(a_position.x, a_position.y);
}
`;

const windowFragmentShaderSource = `#version 300 es
precision highp float;
precision mediump int;
precision highp usampler2D;

in vec2 v_texcoord;

uniform float u_atlaslength;
uniform usampler2D u_tile0texture;
uniform usampler2D u_tile1texture;
uniform usampler2D u_atlastexture;
uniform mat4 u_colors;
uniform int u_control;
uniform vec2 u_offset;

out vec4 outColor;

void main() {
  if ((u_control & 0x80) > 0) {
    uint r0 = texture(u_tile0texture, v_texcoord).r;
    uint r1 = texture(u_tile1texture, v_texcoord).r;
    uint r = r0;
    if ((u_control & 0x40) > 0) {
      // BG uses higher tilemap
      r = r1;
    }
    
    if ((u_control & 0x10) == 0 && r < 128u) {
      r += 256u;
    }

    float tilex = v_texcoord.x * 8.0 * 32.0;
    float tiley = v_texcoord.y * 8.0 * 32.0;
    int offsetx = 7 - int(tilex - (floor(tilex / 8.0) * 8.0));
    float offsety = (tiley - (floor(tiley / 8.0) * 8.0)) / 8.0;
    vec2 atlascoord = vec2(offsety, (float(r) + 0.5) / u_atlaslength);
    uint high = texture(u_atlastexture, atlascoord).r;
    uint low = texture(u_atlastexture, atlascoord).g;

    uint hbit = (high >> offsetx) & 1u;
    uint lbit = (low >> offsetx) & 1u;
    uint color = hbit | (lbit << 1);

    outColor = u_colors[color];
  } else {
    outColor = vec4(0, 0, 0, 0);
  }
}
`;

const spriteVertexShaderSource = `#version 300 es
precision mediump int;

in vec4 a_position;

uniform vec2 u_offset;
uniform int u_control;

out vec2 v_texcoord;

void main() {
  // x: convert 8-168 into -1.0 to 1.0
  // y: convert 16-160 into -1.0 to 1.0
  float yScale = 1.0;
  if ((u_control & 0x4) > 0) {
    yScale = 2.0;
  }
  gl_Position = vec4(
    (a_position.x + u_offset.x - 88.) / 80.,
    (a_position.y * yScale + u_offset.y - 88.) / -72.,
    0,
    1
  );
  v_texcoord = vec2(a_position.x, a_position.y * yScale);
}
`;

const spriteFragmentShaderSource = `#version 300 es
precision mediump int;
precision highp float;
precision highp usampler2D;

in vec2 v_texcoord;
uniform float u_atlaslength;
uniform usampler2D u_atlastexture;
uniform int u_sprite;
uniform mat4 u_colors;
uniform int u_options;
uniform int u_control;

out vec4 outColor;

void main() {
  float y = v_texcoord.y;
  vec2 atlascoord = vec2(0.0, 0.0);
  if ((u_control & 0x4) > 0) {
    if ((u_options & 0x40) > 0) {
      y = 16.0 - y;
    }
    int base = u_sprite & 0xfe;
    if (y > 8.0) {
      y -= 8.0;
      base += 1;
    }
    atlascoord.x = y / 8.0;
    atlascoord.y = float(base) / u_atlaslength;
  } else {
    if ((u_options & 0x40) > 0) {
      y = 8. - y;
    }
    atlascoord.x = y / 8.;
    atlascoord.y = float(u_sprite) / u_atlaslength;
  }

  uint high = texture(u_atlastexture, atlascoord).r;
  uint low = texture(u_atlastexture, atlascoord).g;
  int x = int(v_texcoord.x);
  if ((u_options & 0x20) == 0) {
    x = 7 - x;
  }
  uint hbit = (high >> x) & 1u;
  uint lbit = (low >> x) & 1u;
  uint color = hbit | (lbit << 1);

  outColor = u_colors[color];
}
`;

function createShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    return shader;
  }
  console.error(gl.getShaderInfoLog(shader));
  gl.deleteShader(shader);
}

function createProgram(gl, vertexShader, fragmentShader) {
  const program = gl.createProgram();
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  if (gl.getProgramParameter(program, gl.LINK_STATUS)) {
    return program;
  }
  console.error(gl.getProgramInfoLog(program));
  gl.deleteProgram(program);
}

function updateGLColors(palette, colors, skipZero) {
  const c0 = palette & 3;
  const c1 = (palette >> 2) & 3;
  const c2 = (palette >> 4) & 3;
  const c3 = (palette >> 6) & 3;
  if (!skipZero) {
    colors[0] = GL_COLORS[c0][0];
    colors[1] = GL_COLORS[c0][1];
    colors[2] = GL_COLORS[c0][2];
  }

  colors[4] = GL_COLORS[c1][0];
  colors[5] = GL_COLORS[c1][1];
  colors[6] = GL_COLORS[c1][2];

  colors[8] = GL_COLORS[c2][0];
  colors[9] = GL_COLORS[c2][1];
  colors[10] = GL_COLORS[c2][2];

  colors[12] = GL_COLORS[c3][0];
  colors[13] = GL_COLORS[c3][1];
  colors[14] = GL_COLORS[c3][2];
}

class Graphics {
  constructor(gl) {
    this.gl = gl;
    gl.enable(gl.BLEND);

    const bgProgram = createProgram(
      gl,
      createShader(gl, gl.VERTEX_SHADER, tileVertexShaderSource),
      createShader(gl, gl.FRAGMENT_SHADER, tileFragmentShaderSource)
    );
    this.bg = {
      program: bgProgram,
      attributes: {
        position: gl.getAttribLocation(bgProgram, 'a_position'),
      },
      uniforms: {
        atlasLength: gl.getUniformLocation(bgProgram, 'u_atlaslength'),
        atlasTex: gl.getUniformLocation(bgProgram, 'u_atlastexture'),
        colors: gl.getUniformLocation(bgProgram, 'u_colors'),
        control: gl.getUniformLocation(bgProgram, 'u_control'),
        offset: gl.getUniformLocation(bgProgram, 'u_offset'),
        tile0Tex: gl.getUniformLocation(bgProgram, 'u_tile0texture'),
        tile1Tex: gl.getUniformLocation(bgProgram, 'u_tile1texture'),
      },
    };

    const windowProgram = createProgram(
      gl,
      createShader(gl, gl.VERTEX_SHADER, windowVertexShaderSource),
      createShader(gl, gl.FRAGMENT_SHADER, windowFragmentShaderSource),
    );
    this.window = {
      program: windowProgram,
      attributes: {
        position: gl.getAttribLocation(windowProgram, 'a_position'),
      },
      uniforms: {
        atlasLength: gl.getUniformLocation(windowProgram, 'u_atlaslength'),
        atlasTex: gl.getUniformLocation(windowProgram, 'u_atlastexture'),
        colors: gl.getUniformLocation(windowProgram, 'u_colors'),
        control: gl.getUniformLocation(windowProgram, 'u_control'),
        offset: gl.getUniformLocation(windowProgram, 'u_offset'),
        tile0Tex: gl.getUniformLocation(windowProgram, 'u_tile0texture'),
        tile1Tex: gl.getUniformLocation(windowProgram, 'u_tile1texture'),
      },
    };

    const positionBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    // Big quad across the screen
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
      0, 0, 0, 1, 1, 0,
      1, 0, 0, 1, 1, 1,
    ]), gl.STATIC_DRAW);

    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    gl.enableVertexAttribArray(this.bg.attributes.position);
    gl.vertexAttribPointer(this.bg.attributes.position, 2, gl.FLOAT, false, 0, 0);

    gl.enableVertexAttribArray(this.window.attributes.position);
    gl.vertexAttribPointer(this.window.attributes.position, 2, gl.FLOAT, false, 0, 0);

    this.tile0Texture = gl.createTexture();
    this.tile1Texture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0 + 0);
    gl.bindTexture(gl.TEXTURE_2D, this.tile0Texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
    gl.activeTexture(gl.TEXTURE0 + 2);
    gl.bindTexture(gl.TEXTURE_2D, this.tile1Texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);

    this.atlasTexture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0 + 4);
    gl.bindTexture(gl.TEXTURE_2D, this.atlasTexture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    const spriteProgram = createProgram(
      gl,
      createShader(gl, gl.VERTEX_SHADER, spriteVertexShaderSource),
      createShader(gl, gl.FRAGMENT_SHADER, spriteFragmentShaderSource),
    );
    this.sprite = {
      program: spriteProgram,
      attributes: {
        position: gl.getAttribLocation(spriteProgram, 'a_position'),
      },
      uniforms: {
        atlasLength: gl.getUniformLocation(spriteProgram, 'u_atlaslength'),
        atlasTex: gl.getUniformLocation(spriteProgram, 'u_atlastexture'),
        colors: gl.getUniformLocation(spriteProgram, 'u_colors'),
        control: gl.getUniformLocation(spriteProgram, 'u_control'),
        offset: gl.getUniformLocation(spriteProgram, 'u_offset'),
        options: gl.getUniformLocation(spriteProgram, 'u_options'),
        sprite: gl.getUniformLocation(spriteProgram, 'u_sprite'),
      },
    };
    
    this.sprites = [];
    for (let i = 0; i < SPRITE_COUNT; i++) {
      const positionBuffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, spriteGeom, gl.STATIC_DRAW);

      const vao = gl.createVertexArray();
      gl.bindVertexArray(vao);
      gl.enableVertexAttribArray(this.sprite.attributes.position);
      gl.vertexAttribPointer(this.sprite.attributes.position, 2, gl.FLOAT, false, 0, 0);
      this.sprites.push({
        vao,
        x: 0,
        y: 0,
      });
    }

    this.bgp = [
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
    ];
    this.obp0 = [
      0.0, 0.0, 0.0, 0.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
    ];
    this.obp1 = [
      0.0, 0.0, 0.0, 0.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
    ];
  }

  loadBGMap0(data) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + 0);
    gl.bindTexture(gl.TEXTURE_2D, this.tile0Texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8UI, 32, 32, 0, gl.RED_INTEGER, gl.UNSIGNED_BYTE, data);
  }

  loadBGMap1(data) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + 2);
    gl.bindTexture(gl.TEXTURE_2D, this.tile1Texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8UI, 32, 32, 0, gl.RED_INTEGER, gl.UNSIGNED_BYTE, data);
  }

  loadBGTiles(data0, data1) {
    // data should be 1024 bytes, 32 rows of 32 bytes
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + 0);
    gl.bindTexture(gl.TEXTURE_2D, this.tile0Texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8UI, 32, 32, 0, gl.RED_INTEGER, gl.UNSIGNED_BYTE, data0);

    gl.activeTexture(gl.TEXTURE0 + 2);
    gl.bindTexture(gl.TEXTURE_2D, this.tile1Texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8UI, 32, 32, 0, gl.RED_INTEGER, gl.UNSIGNED_BYTE, data1);
  }

  loadTileData(data) {
    // data should be 384 tiles, each 16 bytes long
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + 4);
    gl.bindTexture(gl.TEXTURE_2D, this.atlasTexture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RG8UI, 8, 384, 0, gl.RG_INTEGER, gl.UNSIGNED_BYTE, data);
  }

  draw(spriteTable, zeroPage) {
    updateGLColors(zeroPage[0x47], this.bgp, false);
    updateGLColors(zeroPage[0x48], this.obp0, true);
    updateGLColors(zeroPage[0x49], this.obp1, true);
    this._draw(zeroPage[0x40], zeroPage[0x43], zeroPage[0x42], zeroPage[0x4b], zeroPage[0x4a]);
  }

  _draw(control, offsetX, offsetY, windowX, windowY) {
    const colors = this.bgp;
    const obp0 = this.obp0;
    const obp1 = this.obp1;
    const gl = this.gl;
    gl.viewport(0, 0, 320, 288);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    if (control & 0x1) {
      // BG is enabled
      gl.useProgram(this.bg.program);
      gl.bindVertexArray(this.vao);

      gl.uniform1f(this.bg.uniforms.atlasLength, 384);
      gl.uniform1i(this.bg.uniforms.atlasTex, 4);
      gl.uniformMatrix4fv(this.bg.uniforms.colors, false, colors);
      gl.uniform1i(this.bg.uniforms.control, control);
      gl.uniform2f(this.bg.uniforms.offset, offsetX / 256, offsetY / 256);
      gl.uniform1i(this.bg.uniforms.tile0Tex, 0);
      gl.uniform1i(this.bg.uniforms.tile1Tex, 2);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    if (control & 0x20) {
      // Window is enabled
      gl.useProgram(this.window.program);
      gl.bindVertexArray(this.vao);

      gl.uniform1f(this.window.uniforms.atlasLength, 384);
      gl.uniform1i(this.window.uniforms.atlasTex, 4);
      gl.uniformMatrix4fv(this.window.uniforms.colors, false, colors);
      gl.uniform1i(this.window.uniforms.control, control);
      gl.uniform2f(this.window.uniforms.offset, windowX, windowY);
      gl.uniform1i(this.window.uniforms.tile0Tex, 0);
      gl.uniform1i(this.window.uniforms.tile1Tex, 2);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    if (control & 0x2) {
      // Sprites are enabled
      gl.useProgram(this.sprite.program);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      for (let i = 0; i < this.sprites.length; i++) {
        const sprite = this.sprites[i];
        if (sprite.x === 0 || sprite.y === 0) {
          continue;
        }
        const options = sprite.options;
        gl.bindVertexArray(sprite.vao);
        gl.uniform2f(this.sprite.uniforms.offset, sprite.x, sprite.y);
        gl.uniform1f(this.sprite.uniforms.atlasLength, 384);
        gl.uniform1i(this.sprite.uniforms.atlasTex, 4);
        gl.uniform1i(this.sprite.uniforms.sprite, sprite.index);
        gl.uniform1i(this.sprite.uniforms.options, sprite.options);
        gl.uniform1i(this.sprite.uniforms.control, control);
        gl.uniformMatrix4fv(this.sprite.uniforms.colors, false, options & 0x10 ? obp1 : obp0);
        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }
    }
  }

  loadOAMData(data) {
    for (let i = 0; i < this.sprites.length; i++) {
      const s = this.sprites[i];
      s.y = data[i * 4];
      s.x = data[i * 4 + 1];
      s.index = data[i * 4 + 2];
      s.options = data[i * 4 + 3];
    }
  }
}

Graphics.createShader = createShader;
Graphics.createProgram = createProgram;

window.Graphics = Graphics;

})();
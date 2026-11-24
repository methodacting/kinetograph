// size: 48 16
// One array of 16 bytes uploaded three ways: as a 4 x 1 and a 2 x 2 RGBA8 texture and as a 1 x 1 RGBA32F one, so the
// recording names one blob under three descriptions. The thirds of the canvas show the three textures, sampled NEAREST,
// with alpha 1.
const p = gl.createProgram();
for (const [type, src] of [
  [gl.VERTEX_SHADER, '#version 300 es\nin vec2 a_pos; out vec2 v_uv; void main() { v_uv = a_pos * 0.5 + 0.5; gl_Position = vec4(a_pos, 0.0, 1.0); }'],
  [gl.FRAGMENT_SHADER, '#version 300 es\nprecision highp float; in vec2 v_uv; uniform sampler2D u_t; out vec4 o; void main() { o = vec4(texture(u_t, v_uv).rgb, 1.0); }'],
]) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  gl.attachShader(p, s);
}
gl.bindAttribLocation(p, 0, 'a_pos');
gl.linkProgram(p);
gl.useProgram(p);
// As RGBA8: (200, 0, 128), (0, 200, 0), (0, 0, 200), (0, 0, 0). As floats: about (1.0, 0.503, 0.391, 0).
const bytes = new Uint8Array([200, 0, 128, 63, 0, 200, 0, 63, 0, 0, 200, 62, 0, 0, 0, 0]);
function texture(width, height, upload) {
  const t = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, t);
  upload(width, height);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
const rgba8 = (w, h) => gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, bytes);
const wide = texture(4, 1, rgba8);
const square = texture(2, 2, rgba8);
const floats = texture(1, 1, () =>
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, 1, 1, 0, gl.RGBA, gl.FLOAT, new Float32Array(bytes.buffer)),
);
gl.uniform1i(gl.getUniformLocation(p, 'u_t'), 0);
const b = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, b);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0);
gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
gl.activeTexture(gl.TEXTURE0);
[wide, square, floats].forEach((t, i) => {
  gl.bindTexture(gl.TEXTURE_2D, t);
  gl.viewport(16 * i, 0, 16, 16);
  gl.drawArrays(gl.TRIANGLES, 0, 6);
});

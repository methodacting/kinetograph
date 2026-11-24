// size: 16 16
// A later frame's recording that samples a target an earlier frame drew in. The first frame draws a 2 x 1 texture
// (red, blue) into a 2 x 1 target and lays green over its right texel with texSubImage2D; the next (`gl.resetState()`,
// the engine's frame boundary) only samples the target onto the canvas, so its recording carries the draw and the
// upload.
const p = gl.createProgram();
for (const [type, src] of [
  [gl.VERTEX_SHADER, 'attribute vec2 a_pos; varying vec2 v_uv; void main() { v_uv = a_pos * 0.5 + 0.5; gl_Position = vec4(a_pos, 0.0, 1.0); }'],
  [gl.FRAGMENT_SHADER, 'precision mediump float; varying vec2 v_uv; uniform sampler2D u_t; void main() { gl_FragColor = vec4(texture2D(u_t, v_uv).rgb, 1.0); }'],
]) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  gl.attachShader(p, s);
}
gl.bindAttribLocation(p, 0, 'a_pos');
gl.linkProgram(p);
function nearest() {
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}
const picture = gl.createTexture();
gl.bindTexture(gl.TEXTURE_2D, picture);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 2, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 0, 0, 255, 0, 0, 255, 255]));
nearest();
const target = gl.createTexture();
gl.bindTexture(gl.TEXTURE_2D, target);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 2, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
nearest();
const framebuffer = gl.createFramebuffer();
gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer);
gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, target, 0);
const b = gl.createBuffer();
function quad(texture) {
  gl.useProgram(p);
  gl.uniform1i(gl.getUniformLocation(p, 'u_t'), 0);
  gl.bindBuffer(gl.ARRAY_BUFFER, b);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.drawArrays(gl.TRIANGLES, 0, 6);
}
gl.bindBuffer(gl.ARRAY_BUFFER, b);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
gl.viewport(0, 0, 2, 1);
quad(picture);
gl.bindTexture(gl.TEXTURE_2D, target);
gl.texSubImage2D(gl.TEXTURE_2D, 0, 1, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([0, 255, 0, 255]));
if (gl.resetState) gl.resetState();
gl.bindFramebuffer(gl.FRAMEBUFFER, null);
gl.viewport(0, 0, W, H);
quad(target);

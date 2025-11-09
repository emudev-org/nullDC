/**
 * WebGL-based renderer for SGC waveform visualization
 * Renders to an offscreen canvas and can blit to multiple destination canvases
 */
export class SgcWaveformRenderer {
  private offscreenCanvas: OffscreenCanvas;
  private gl: WebGLRenderingContext;
  private program: WebGLProgram;
  private width: number = 0;
  private height: number = 0;

  constructor() {
    // Create offscreen canvas with initial size of 1x1
    this.offscreenCanvas = new OffscreenCanvas(1, 1);

    // Get WebGL context
    const glContext = this.offscreenCanvas.getContext('webgl');
    if (!glContext) {
      throw new Error('WebGL is not available');
    }
    this.gl = glContext as WebGLRenderingContext;

    // Initialize shaders and program
    this.program = this.initializeProgram();

    // Enable blending for transparency
    this.gl.enable(this.gl.BLEND);
    this.gl.blendFunc(this.gl.SRC_ALPHA, this.gl.ONE_MINUS_SRC_ALPHA);
  }

  private initializeProgram(): WebGLProgram {
    const gl = this.gl;

    const vertexShaderSource = `
      attribute vec2 a_position; // x: sample index [0-numFrames), y: normalized Y [0, 1]
      attribute vec4 a_color;
      uniform vec2 u_resolution;
      uniform vec2 u_transform; // x: scrollOffsetX, y: zoomLevel
      uniform float u_numFrames; // Number of frames in the data
      varying vec4 v_color;

      void main() {
        // Convert sample index to normalized X [0, 1]
        float normalizedX = a_position.x / u_numFrames;

        // Convert to pixel coordinates
        float pixelX = normalizedX * u_resolution.x;
        float pixelY = a_position.y * u_resolution.y;

        // Apply pan and zoom (only to X)
        vec2 transformed = vec2(
          (pixelX * u_transform.y) - u_transform.x,
          pixelY
        );

        // Convert from pixel coordinates to clip space (-1 to 1)
        vec2 clipSpace = (transformed / u_resolution) * 2.0 - 1.0;
        clipSpace.y = -clipSpace.y; // Flip Y axis

        gl_Position = vec4(clipSpace, 0.0, 1.0);
        v_color = a_color;
      }
    `;

    const fragmentShaderSource = `
      precision mediump float;
      varying vec4 v_color;

      void main() {
        gl_FragColor = v_color;
      }
    `;

    // Compile shaders
    const compileShader = (type: number, source: string): WebGLShader => {
      const shader = gl.createShader(type);
      if (!shader) {
        throw new Error('Failed to create shader');
      }

      gl.shaderSource(shader, source);
      gl.compileShader(shader);

      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        const error = gl.getShaderInfoLog(shader);
        gl.deleteShader(shader);
        throw new Error(`Shader compile error: ${error}`);
      }

      return shader;
    };

    const vertexShader = compileShader(gl.VERTEX_SHADER, vertexShaderSource);
    const fragmentShader = compileShader(gl.FRAGMENT_SHADER, fragmentShaderSource);

    // Link program
    const program = gl.createProgram();
    if (!program) {
      throw new Error('Failed to create program');
    }

    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      const error = gl.getProgramInfoLog(program);
      gl.deleteProgram(program);
      throw new Error(`Program link error: ${error}`);
    }

    // Cleanup shaders (they're no longer needed after linking)
    gl.deleteShader(vertexShader);
    gl.deleteShader(fragmentShader);

    return program;
  }

  /**
   * Create a vertex buffer from Float32Array data
   */
  createVertexBuffer(data: Float32Array): WebGLBuffer {
    const gl = this.gl;
    const buffer = gl.createBuffer();
    if (!buffer) {
      throw new Error('Failed to create buffer');
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);

    return buffer;
  }

  /**
   * Destroy a vertex buffer
   */
  destroyVertexBuffer(buffer: WebGLBuffer): void {
    this.gl.deleteBuffer(buffer);
  }

  /**
   * Clear the offscreen canvas
   * Automatically resizes to match destination canvas if needed
   */
  clear(width: number, height: number): void {
    // Resize if dimensions don't match
    if (this.width !== width || this.height !== height) {
      this.width = width;
      this.height = height;
      this.offscreenCanvas.width = width;
      this.offscreenCanvas.height = height;
    }

    const gl = this.gl;
    gl.viewport(0, 0, this.width, this.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  /**
   * Render vertex buffer to the offscreen canvas
   */
  render(
    vertexBuffer: WebGLBuffer,
    vertexCount: number,
    scrollOffsetX: number = 0,
    zoomLevel: number = 1,
    numFrames: number = 1024
  ): void {
    const gl = this.gl;
    const program = this.program;

    // Set viewport
    gl.viewport(0, 0, this.width, this.height);

    // Use program
    gl.useProgram(program);

    // Set uniforms
    const u_resolution = gl.getUniformLocation(program, 'u_resolution');
    const u_transform = gl.getUniformLocation(program, 'u_transform');
    const u_numFrames = gl.getUniformLocation(program, 'u_numFrames');
    gl.uniform2f(u_resolution, this.width, this.height);
    gl.uniform2f(u_transform, scrollOffsetX, zoomLevel);
    gl.uniform1f(u_numFrames, numFrames);

    // Bind buffer and set up attributes
    const a_position = gl.getAttribLocation(program, 'a_position');
    const a_color = gl.getAttribLocation(program, 'a_color');

    gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
    gl.enableVertexAttribArray(a_position);
    gl.vertexAttribPointer(a_position, 2, gl.FLOAT, false, 24, 0);
    gl.enableVertexAttribArray(a_color);
    gl.vertexAttribPointer(a_color, 4, gl.FLOAT, false, 24, 8);

    // Draw
    gl.drawArrays(gl.TRIANGLES, 0, vertexCount);
  }

  /**
   * Copy the offscreen canvas to a destination canvas
   */
  copyToCanvas(destinationCanvas: HTMLCanvasElement): void {
    const ctx = destinationCanvas.getContext('2d');
    if (!ctx) {
      console.warn('Failed to get 2D context from destination canvas');
      return;
    }

    // Transfer offscreen canvas to ImageBitmap and draw
    const imageBitmap = this.offscreenCanvas.transferToImageBitmap();
    ctx.clearRect(0, 0, destinationCanvas.width, destinationCanvas.height);
    ctx.drawImage(imageBitmap, 0, 0);
  }

  /**
   * Resize the renderer
   */
  resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    this.offscreenCanvas.width = width;
    this.offscreenCanvas.height = height;
  }

  /**
   * Get current dimensions
   */
  getDimensions(): { width: number; height: number } {
    return { width: this.width, height: this.height };
  }

  /**
   * Cleanup resources
   */
  destroy(): void {
    this.gl.deleteProgram(this.program);
  }
}

/**
 * C-style preprocessor for DSP compiler
 * Handles #define directives and macro expansion
 */

export interface PreprocessorError {
  line: number;
  message: string;
}

export class PreprocessorException extends Error {
  constructor(public errors: PreprocessorError[]) {
    super(errors.map(e => `Line ${e.line}: ${e.message}`).join('\n'));
    this.name = 'PreprocessorException';
  }
}

export interface MacroDefinition {
  name: string;
  value: string;
  line: number;
}

export function preprocessDspSource(source: string): {
  output: string;
  errors: PreprocessorError[];
  macros: Map<string, MacroDefinition>;
} {
  // First pass: remove C-style block comments /* ... */
  let processedSource = source;
  processedSource = processedSource.replace(/\/\*[\s\S]*?\*\//g, (match) => {
    // Replace with same number of newlines to preserve line numbers
    return match.split('\n').map((_, i) => i === 0 ? '' : '\n').join('');
  });

  const lines = processedSource.split('\n');
  const macros = new Map<string, MacroDefinition>();
  const errors: PreprocessorError[] = [];
  const outputLines: string[] = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    let line = lines[lineIndex];
    const lineNum = lineIndex + 1;

    // Remove C++ style comments from the line
    const commentIndex = line.indexOf('//');
    if (commentIndex !== -1) {
      line = line.substring(0, commentIndex);
    }

    // Trim the line (remove leading/trailing whitespace)
    line = line.trim();

    // Skip empty lines
    if (line === '') {
      outputLines.push('');
      continue;
    }

    // Handle #define directive
    const defineMatch = /^\s*#define\s+([a-zA-Z_]\w*)\s+(.+)$/.exec(line);
    if (defineMatch) {
      const macroName = defineMatch[1];
      const macroValue = defineMatch[2].trim();

      // Check if macro is already defined
      if (macros.has(macroName)) {
        errors.push({
          line: lineNum,
          message: `Macro '${macroName}' redefined (previously defined on line ${macros.get(macroName)!.line})`
        });
      }

      macros.set(macroName, {
        name: macroName,
        value: macroValue,
        line: lineNum
      });

      // Convert to comment in output
      outputLines.push(`// ${line.trim()}`);
      continue;
    }

    // Handle regular comments (pass through)
    if (/^\s*(\/\/|#)/.test(line)) {
      outputLines.push(line);
      continue;
    }

    // Expand macros in the line
    let expandedLine = line;
    let hasChanges = true;
    let iterations = 0;
    const maxIterations = 100; // Prevent infinite loops

    while (hasChanges && iterations < maxIterations) {
      hasChanges = false;
      iterations++;

      // Try to replace each macro
      for (const [macroName, macroDef] of macros) {
        // Use word boundary regex to match whole identifiers only
        const regex = new RegExp(`\\b${macroName}\\b`, 'g');
        const newLine = expandedLine.replace(regex, macroDef.value);

        if (newLine !== expandedLine) {
          expandedLine = newLine;
          hasChanges = true;
        }
      }
    }

    if (iterations >= maxIterations) {
      errors.push({
        line: lineNum,
        message: `Macro expansion exceeded maximum iterations (possible circular definition)`
      });
    }

    outputLines.push(expandedLine);
  }

  return {
    output: outputLines.join('\n'),
    errors,
    macros
  };
}

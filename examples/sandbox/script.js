// Colour Palette Generator
console.log('Colour Palette Generator loaded');

// Get all swatch elements
const swatches = document.querySelectorAll('.swatch');

// Log swatch count for verification
console.log(`Found ${swatches.length} swatch elements`);

/**
 * Generates a random valid hex colour code in #RRGGBB format.
 * @returns {string} A hex colour code (e.g., "#A1B2C3")
 */
function generateRandomColour() {
  const randomInt = Math.floor(Math.random() * 0xFFFFFF);
  const hex = randomInt.toString(16).padStart(6, '0').toUpperCase();
  return `#${hex}`;
}

/**
 * Applies a colour to a swatch element.
 * @param {HTMLElement} swatch - The swatch element
 * @param {string} colour - The hex colour code
 */
function applyColourToSwatch(swatch, colour) {
  const colourElement = swatch.querySelector('.swatch-colour');
  const hexCodeElement = swatch.querySelector('.hex-code');
  
  if (colourElement) {
    colourElement.style.backgroundColor = colour;
  }
  if (hexCodeElement) {
    hexCodeElement.textContent = colour;
    // Store hex code in data attribute for reliable clipboard access
    hexCodeElement.dataset.hexCode = colour;
  }
}

/**
 * Generates and applies random colours to all unlocked swatches.
 * Locked swatches retain their current colour.
 */
function generatePalette() {
  swatches.forEach((swatch) => {
    if (!isLocked(swatch)) {
      const colour = generateRandomColour();
      applyColourToSwatch(swatch, colour);
    }
  });
}

/**
 * Toggles the locked state of a swatch.
 * @param {HTMLElement} swatch - The swatch element to toggle
 */
function toggleLock(swatch) {
  swatch.classList.toggle('locked');
}

/**
 * Checks if a swatch is locked.
 * @param {HTMLElement} swatch - The swatch element to check
 * @returns {boolean} True if the swatch is locked
 */
function isLocked(swatch) {
  return swatch.classList.contains('locked');
}

// Add click handlers to toggle lock state
swatches.forEach((swatch) => {
  swatch.addEventListener('click', (event) => {
    // Don't toggle lock if hex code was clicked
    if (event.target.classList.contains('hex-code')) {
      return;
    }
    toggleLock(swatch);
    updateUrlHash();
  });
});

/**
 * Copies text to the system clipboard.
 * @param {string} text - The text to copy
 * @returns {Promise<boolean>} True if copy succeeded
 */
async function copyToClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch (err) {
    console.error('Failed to copy to clipboard:', err);
    return false;
  }
}

/**
 * Shows copy feedback on a hex code element.
 * @param {HTMLElement} hexCodeElement - The hex code element
 */
function showCopyFeedback(hexCodeElement) {
  const originalText = hexCodeElement.textContent;
  hexCodeElement.textContent = 'Copied!';
  hexCodeElement.classList.add('copied');
  
  setTimeout(() => {
    hexCodeElement.textContent = originalText;
    hexCodeElement.classList.remove('copied');
  }, 1500);
}

// Add click handlers to hex codes for copy functionality
document.querySelectorAll('.hex-code').forEach((hexCode) => {
  hexCode.addEventListener('click', async (event) => {
    event.stopPropagation(); // Prevent swatch lock toggle
    // Read from data attribute to avoid copying "Copied!" text during feedback
    const text = hexCode.dataset.hexCode || hexCode.textContent;
    const success = await copyToClipboard(text);
    if (success) {
      showCopyFeedback(hexCode);
    }
  });
});

// Add keyboard handler for spacebar to regenerate palette
document.addEventListener('keydown', (event) => {
  if (event.code === 'Space') {
    // Prevent default scrolling behaviour
    event.preventDefault();
    generatePalette();
    updateUrlHash();
  }
});

// Generate initial palette on page load
generatePalette();
updateUrlHash();

// Export functionality
const exportBtn = document.getElementById('export-btn');
const exportDropdown = document.getElementById('export-dropdown');

/**
 * Gets the current palette as an array of hex codes.
 * @returns {string[]} Array of 5 hex colour codes
 */
function getCurrentPalette() {
  const palette = [];
  swatches.forEach((swatch) => {
    const hexCode = swatch.querySelector('.hex-code').textContent;
    palette.push(hexCode);
  });
  return palette;
}

/**
 * Downloads a file with the given content.
 * @param {string} content - The file content
 * @param {string} filename - The filename to save as
 * @param {string} mimeType - The MIME type of the file
 */
function downloadFile(content, filename, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/**
 * Exports the current palette as JSON.
 */
function exportAsJSON() {
  const palette = getCurrentPalette();
  const data = {
    palette: palette,
    exportedAt: new Date().toISOString()
  };
  const json = JSON.stringify(data, null, 2);
  downloadFile(json, 'palette.json', 'application/json');
}

/**
 * Exports the current palette as CSS variables.
 */
function exportAsCSS() {
  const palette = getCurrentPalette();
  const cssLines = [':root {'];
  palette.forEach((colour, index) => {
    cssLines.push(`  --colour-${index + 1}: ${colour};`);
  });
  cssLines.push('}');
  const css = cssLines.join('\n');
  downloadFile(css, 'palette.css', 'text/css');
}

// Toggle export dropdown
exportBtn.addEventListener('click', (event) => {
  event.stopPropagation();
  exportDropdown.classList.toggle('open');
});

// Handle export format selection
document.querySelectorAll('.export-option').forEach((option) => {
  option.addEventListener('click', (event) => {
    event.stopPropagation();
    const format = option.dataset.format;
    if (format === 'json') {
      exportAsJSON();
    } else if (format === 'css') {
      exportAsCSS();
    }
    exportDropdown.classList.remove('open');
  });
});

// Close dropdown when clicking outside
document.addEventListener('click', () => {
  exportDropdown.classList.remove('open');
});

// URL Hash Encoding
/**
 * Encodes the current palette state (colours and lock status) into the URL hash.
 * Format: RRGGBB[L]-RRGGBB[L]-... where L suffix indicates locked
 */
function encodeToHash() {
  const parts = [];
  swatches.forEach((swatch) => {
    const hexCode = swatch.querySelector('.hex-code').dataset.hexCode || 
                    swatch.querySelector('.hex-code').textContent;
    // Remove # and add L suffix if locked
    const colourPart = hexCode.replace('#', '');
    const lockSuffix = isLocked(swatch) ? 'L' : '';
    parts.push(colourPart + lockSuffix);
  });
  return parts.join('-');
}

/**
 * Updates the URL hash to reflect the current palette state.
 */
function updateUrlHash() {
  const hash = encodeToHash();
  window.location.hash = hash;
}

// Expose to window for browser console access
window.generateRandomColour = generateRandomColour;
window.generatePalette = generatePalette;
window.toggleLock = toggleLock;
window.isLocked = isLocked;
window.getCurrentPalette = getCurrentPalette;
window.exportAsJSON = exportAsJSON;
window.exportAsCSS = exportAsCSS;
window.copyToClipboard = copyToClipboard;
window.encodeToHash = encodeToHash;
window.updateUrlHash = updateUrlHash;

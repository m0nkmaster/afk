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
    const text = hexCode.textContent;
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
  }
});

// Generate initial palette on page load
generatePalette();

// Expose to window for browser console access
window.generateRandomColour = generateRandomColour;
window.generatePalette = generatePalette;
window.toggleLock = toggleLock;
window.isLocked = isLocked;

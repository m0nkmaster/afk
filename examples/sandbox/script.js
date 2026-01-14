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
  swatch.addEventListener('click', () => {
    toggleLock(swatch);
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

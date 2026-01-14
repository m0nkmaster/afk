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
 * Generates and applies random colours to all swatches.
 */
function generatePalette() {
  swatches.forEach((swatch) => {
    const colour = generateRandomColour();
    applyColourToSwatch(swatch, colour);
  });
}

// Generate initial palette on page load
generatePalette();

// Expose to window for browser console access
window.generateRandomColour = generateRandomColour;
window.generatePalette = generatePalette;

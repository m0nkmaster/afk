// Colour Palette Generator
console.log('Colour Palette Generator loaded');

// Get all swatch elements
const swatches = document.querySelectorAll('.swatch');

// Log swatch count for verification
console.log(`Found ${swatches.length} swatch elements`);

// Log each swatch with its index
swatches.forEach((swatch, index) => {
  console.log(`Swatch ${index}: data-index="${swatch.dataset.index}"`);
});

/**
 * Generates a random valid hex colour code in #RRGGBB format.
 * @returns {string} A hex colour code (e.g., "#A1B2C3")
 */
function generateRandomColour() {
  const randomInt = Math.floor(Math.random() * 0xFFFFFF);
  const hex = randomInt.toString(16).padStart(6, '0').toUpperCase();
  return `#${hex}`;
}

// Expose to window for browser console access
window.generateRandomColour = generateRandomColour;

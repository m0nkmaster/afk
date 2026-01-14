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

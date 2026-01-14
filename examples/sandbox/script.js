// Memory Card Game
console.log('Memory Card Game loaded');

// Symbols for the 8 pairs of cards
const CARD_SYMBOLS = ['🍎', '🍊', '🍋', '🍇', '🍓', '🍒', '🥝', '🍑'];

// Create card objects - 8 pairs (16 cards total)
function createCards() {
  const cards = [];
  
  // Create pairs of cards with matching symbols
  CARD_SYMBOLS.forEach((symbol, index) => {
    // First card of the pair
    cards.push({
      id: index * 2,
      symbol: symbol,
      isFlipped: false
    });
    // Second card of the pair
    cards.push({
      id: index * 2 + 1,
      symbol: symbol,
      isFlipped: false
    });
  });
  
  return cards;
}

// Game state object to track all game data
const gameState = {
  cards: createCards(),           // Array of 16 card objects
  flippedCards: [],               // Currently flipped cards (max 2)
  matchedPairs: 0,                // Number of matched pairs found
  moves: 0,                       // Number of moves (turns) taken
  isLocked: false                 // Whether interaction is locked
};

// Log game state for verification
console.log('Game state initialised:', gameState);
console.log('Cards array:', gameState.cards);
console.log('Number of cards:', gameState.cards.length);

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

// Fisher-Yates shuffle algorithm for random card arrangement
function shuffleCards(cards) {
  const shuffled = [...cards];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled;
}

// Game state object to track all game data
const gameState = {
  cards: shuffleCards(createCards()), // Array of 16 card objects (shuffled)
  flippedCards: [],                   // Currently flipped cards (max 2)
  matchedPairs: 0,                    // Number of matched pairs found
  moves: 0,                           // Number of moves (turns) taken
  isLocked: false,                    // Whether interaction is locked
  timerInterval: null,                // Timer interval reference
  elapsedSeconds: 0                   // Elapsed time in seconds
};

// Render all cards to the DOM
function renderCards() {
  const gameBoard = document.getElementById('game-board');
  gameBoard.innerHTML = '';
  
  gameState.cards.forEach(card => {
    const cardElement = document.createElement('div');
    cardElement.className = 'card';
    cardElement.dataset.id = card.id;
    
    // Add flipped class if card is flipped
    if (card.isFlipped) {
      cardElement.classList.add('flipped');
    }
    
    // Create card face (shows symbol when flipped)
    const cardFace = document.createElement('span');
    cardFace.className = 'card-face';
    cardFace.textContent = card.symbol;
    cardElement.appendChild(cardFace);
    
    // Add click handler
    cardElement.addEventListener('click', () => handleCardClick(card.id));
    
    gameBoard.appendChild(cardElement);
  });
}

// Handle card click events
function handleCardClick(cardId) {
  const card = gameState.cards.find(c => c.id === cardId);
  
  // Ignore clicks on already flipped cards
  if (card.isFlipped) {
    return;
  }
  
  // Flip the card
  card.isFlipped = true;
  
  // Update the DOM
  const cardElement = document.querySelector(`[data-id="${cardId}"]`);
  cardElement.classList.add('flipped');
}

// Reset game to initial state with shuffled cards
function resetGame() {
  // Stop any running timer
  if (gameState.timerInterval) {
    clearInterval(gameState.timerInterval);
    gameState.timerInterval = null;
  }
  
  // Reset all game state
  gameState.cards = shuffleCards(createCards());
  gameState.flippedCards = [];
  gameState.matchedPairs = 0;
  gameState.moves = 0;
  gameState.isLocked = false;
  gameState.elapsedSeconds = 0;
  
  // Re-render the board with new shuffled cards
  renderCards();
  
  console.log('Game reset! Cards shuffled.');
}

// Initialise game on page load
document.addEventListener('DOMContentLoaded', () => {
  renderCards();
  
  // Hook up New Game button
  const newGameBtn = document.getElementById('new-game-btn');
  if (newGameBtn) {
    newGameBtn.addEventListener('click', resetGame);
  }
});

// Log game state for verification
console.log('Game state initialised:', gameState);
console.log('Cards array:', gameState.cards);
console.log('Number of cards:', gameState.cards.length);

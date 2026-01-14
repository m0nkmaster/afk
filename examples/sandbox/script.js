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
      isFlipped: false,
      isMatched: false
    });
    // Second card of the pair
    cards.push({
      id: index * 2 + 1,
      symbol: symbol,
      isFlipped: false,
      isMatched: false
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
  elapsedSeconds: 0,                  // Elapsed time in seconds
  timerStarted: false,                // Whether timer has started
  isGameWon: false                    // Whether the game has been won
};

// Format seconds as m:ss display
function formatTime(seconds) {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

// Update the timer display in the DOM
function updateTimerDisplay() {
  const timerElement = document.getElementById('timer');
  if (timerElement) {
    timerElement.textContent = formatTime(gameState.elapsedSeconds);
  }
}

// Update the moves counter display in the DOM
function updateMovesDisplay() {
  const movesElement = document.getElementById('moves');
  if (movesElement) {
    movesElement.textContent = `Moves: ${gameState.moves}`;
  }
}

// Start the game timer (called on first card flip)
function startTimer() {
  if (gameState.timerStarted) {
    return; // Already started
  }
  
  gameState.timerStarted = true;
  gameState.timerInterval = setInterval(() => {
    gameState.elapsedSeconds++;
    updateTimerDisplay();
  }, 1000);
}

// Render all cards to the DOM
function renderCards() {
  const gameBoard = document.getElementById('game-board');
  gameBoard.innerHTML = '';
  
  gameState.cards.forEach(card => {
    // Create card container
    const cardElement = document.createElement('div');
    cardElement.className = 'card';
    cardElement.dataset.id = card.id;
    
    // Add flipped class if card is flipped
    if (card.isFlipped) {
      cardElement.classList.add('flipped');
    }
    
    // Add matched class if card is matched
    if (card.isMatched) {
      cardElement.classList.add('matched');
    }
    
    // Create inner container for 3D flip
    const cardInner = document.createElement('div');
    cardInner.className = 'card-inner';
    
    // Create front face (face-down side with pattern)
    const cardFront = document.createElement('div');
    cardFront.className = 'card-front';
    
    // Create back face (symbol side)
    const cardBack = document.createElement('div');
    cardBack.className = 'card-back';
    
    // Add symbol to the back
    const cardFace = document.createElement('span');
    cardFace.className = 'card-face';
    cardFace.textContent = card.symbol;
    cardBack.appendChild(cardFace);
    
    // Assemble card structure
    cardInner.appendChild(cardFront);
    cardInner.appendChild(cardBack);
    cardElement.appendChild(cardInner);
    
    // Add click handler
    cardElement.addEventListener('click', () => handleCardClick(card.id));
    
    gameBoard.appendChild(cardElement);
  });
}

// Handle card click events
function handleCardClick(cardId) {
  // Ignore clicks while game is locked (during flip-back delay)
  if (gameState.isLocked) {
    return;
  }
  
  const card = gameState.cards.find(c => c.id === cardId);
  
  // Ignore clicks on already flipped or matched cards
  if (card.isFlipped || card.isMatched) {
    return;
  }
  
  // Start timer on first card flip
  startTimer();
  
  // Flip the card
  card.isFlipped = true;
  gameState.flippedCards.push(card);
  
  // Update the DOM
  const cardElement = document.querySelector(`[data-id="${cardId}"]`);
  cardElement.classList.add('flipped');
  
  // Check for match when two cards are flipped
  if (gameState.flippedCards.length === 2) {
    checkForMatch();
  }
}

// Check if the two flipped cards are a match
function checkForMatch() {
  const [firstCard, secondCard] = gameState.flippedCards;
  
  // Increment moves counter - a turn (pair of cards flipped) is complete
  gameState.moves++;
  updateMovesDisplay();
  
  if (firstCard.symbol === secondCard.symbol) {
    // Match found - mark cards as matched
    handleMatch(firstCard, secondCard);
  } else {
    // No match - flip cards back after a brief delay
    handleNoMatch(firstCard, secondCard);
  }
}

// Handle non-matching cards by flipping them back after a delay
function handleNoMatch(firstCard, secondCard) {
  // Lock the game to prevent further clicks during the delay
  gameState.isLocked = true;
  
  // Wait ~1 second so player can see both cards, then flip back
  setTimeout(() => {
    // Flip cards back to face-down
    firstCard.isFlipped = false;
    secondCard.isFlipped = false;
    
    // Update the DOM - remove flipped class
    const firstElement = document.querySelector(`[data-id="${firstCard.id}"]`);
    const secondElement = document.querySelector(`[data-id="${secondCard.id}"]`);
    firstElement.classList.remove('flipped');
    secondElement.classList.remove('flipped');
    
    // Clear the flipped cards array
    gameState.flippedCards = [];
    
    // Unlock the game for further clicks
    gameState.isLocked = false;
  }, 1000);
}

// Handle a successful match
function handleMatch(firstCard, secondCard) {
  // Mark cards as matched
  firstCard.isMatched = true;
  secondCard.isMatched = true;
  
  // Update the DOM - add matched class
  const firstElement = document.querySelector(`[data-id="${firstCard.id}"]`);
  const secondElement = document.querySelector(`[data-id="${secondCard.id}"]`);
  firstElement.classList.add('matched');
  secondElement.classList.add('matched');
  
  // Increment matched pairs count
  gameState.matchedPairs++;
  
  // Clear the flipped cards array
  gameState.flippedCards = [];
  
  console.log(`Match found! Total pairs: ${gameState.matchedPairs}/8`);
  
  // Check for win condition - all 8 pairs matched
  if (gameState.matchedPairs === 8) {
    handleWin();
  }
}

// Handle game win state
function handleWin() {
  gameState.isGameWon = true;
  console.log('🎉 Congratulations! You won the game!');
  console.log(`Game completed in ${gameState.moves} moves and ${gameState.elapsedSeconds} seconds`);
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
  gameState.timerStarted = false;
  gameState.isGameWon = false;
  
  // Update timer display to show 0:00
  updateTimerDisplay();
  
  // Update moves display to show 0
  updateMovesDisplay();
  
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

// Task List Application

(function() {
    'use strict';

    // Constants
    const STORAGE_KEY = 'taskList';
    const THEME_KEY = 'theme';

    // DOM elements
    const taskInput = document.getElementById('new-task');
    const addButton = document.getElementById('add-task-btn');
    const taskList = document.getElementById('task-list');
    const themeToggle = document.getElementById('theme-toggle');

    /**
     * Applies the given theme and updates the toggle button icon.
     * @param {string} theme - 'dark' or 'light'
     */
    function applyTheme(theme) {
        if (theme === 'light') {
            document.documentElement.setAttribute('data-theme', 'light');
            themeToggle.textContent = '🌙';
            themeToggle.setAttribute('aria-label', 'Switch to dark theme');
        } else {
            document.documentElement.removeAttribute('data-theme');
            themeToggle.textContent = '☀️';
            themeToggle.setAttribute('aria-label', 'Switch to light theme');
        }
    }

    /**
     * Toggles between dark and light themes.
     */
    function toggleTheme() {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';
        applyTheme(newTheme);
        localStorage.setItem(THEME_KEY, newTheme);
    }

    /**
     * Loads the saved theme from localStorage, defaulting to dark.
     */
    function loadTheme() {
        const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
        applyTheme(savedTheme);
    }

    /**
     * Saves the current task list to localStorage.
     */
    function saveTasks() {
        const tasks = [];
        const taskElements = taskList.querySelectorAll('li');
        
        taskElements.forEach(function(li) {
            const textSpan = li.querySelector('.task-text');
            tasks.push({
                text: textSpan.textContent,
                completed: li.classList.contains('completed')
            });
        });
        
        localStorage.setItem(STORAGE_KEY, JSON.stringify(tasks));
    }

    /**
     * Creates a new task list item element.
     * @param {string} text - The task text
     * @param {boolean} [completed=false] - Whether the task is completed
     * @returns {HTMLLIElement} The task list item
     */
    function createTaskElement(text, completed) {
        const li = document.createElement('li');
        
        // Task text span
        const textSpan = document.createElement('span');
        textSpan.className = 'task-text';
        textSpan.textContent = text;
        li.appendChild(textSpan);
        
        // Delete button
        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'delete-btn';
        deleteBtn.setAttribute('aria-label', 'Delete task');
        deleteBtn.textContent = '×';
        deleteBtn.addEventListener('click', function(e) {
            e.stopPropagation(); // Prevent triggering completion toggle
            li.remove();
            saveTasks();
        });
        li.appendChild(deleteBtn);
        
        if (completed) {
            li.classList.add('completed');
        }
        
        // Toggle complete status on click
        li.addEventListener('click', function(e) {
            var wasCompleted = li.classList.contains('completed');
            li.classList.toggle('completed');
            saveTasks();

            // Trigger fireworks only when marking as complete
            if (!wasCompleted) {
                var rect = li.getBoundingClientRect();
                var centreX = rect.left + rect.width / 2;
                var centreY = rect.top + rect.height / 2;
                createFireworks(centreX, centreY);
            }
        });
        
        return li;
    }

    /**
     * Adds a new task to the list.
     */
    function addTask() {
        const text = taskInput.value.trim();
        
        if (!text) {
            return;
        }

        const taskElement = createTaskElement(text);
        taskList.appendChild(taskElement);
        saveTasks();
        
        // Clear the input field
        taskInput.value = '';
        taskInput.focus();
    }

    /**
     * Creates a firework burst effect at the given position.
     * @param {number} x - Centre X position
     * @param {number} y - Centre Y position
     */
    function createFireworks(x, y) {
        const colours = ['#ff6b6b', '#feca57', '#48dbfb', '#ff9ff3', '#54a0ff', '#5f27cd', '#00d2d3', '#1dd1a1'];
        const particleCount = 20;

        for (var i = 0; i < particleCount; i++) {
            var particle = document.createElement('div');
            particle.className = 'firework-particle';

            // Random colour
            particle.style.backgroundColor = colours[Math.floor(Math.random() * colours.length)];

            // Position at burst centre
            particle.style.left = x + 'px';
            particle.style.top = y + 'px';

            // Random direction and distance
            var angle = (Math.PI * 2 * i) / particleCount + (Math.random() - 0.5) * 0.5;
            var distance = 50 + Math.random() * 80;
            var dx = Math.cos(angle) * distance;
            var dy = Math.sin(angle) * distance;

            // Set CSS custom properties for animation end position
            particle.style.setProperty('--dx', dx + 'px');
            particle.style.setProperty('--dy', dy + 'px');

            document.body.appendChild(particle);

            // Remove particle after animation completes
            particle.addEventListener('animationend', function() {
                particle.remove();
            });
        }
    }

    /**
     * Loads tasks from localStorage and displays them in the list.
     */
    function loadTasks() {
        const stored = localStorage.getItem(STORAGE_KEY);
        
        if (!stored) {
            return;
        }
        
        try {
            const tasks = JSON.parse(stored);
            
            tasks.forEach(function(task) {
                const taskElement = createTaskElement(task.text, task.completed);
                taskList.appendChild(taskElement);
            });
        } catch (e) {
            // Invalid JSON in localStorage - ignore and start fresh
            console.warn('Could not parse tasks from localStorage:', e);
        }
    }

    // Event listeners
    addButton.addEventListener('click', addTask);
    themeToggle.addEventListener('click', toggleTheme);
    
    // Allow adding tasks with Enter key
    taskInput.addEventListener('keydown', function(e) {
        if (e.key === 'Enter') {
            addTask();
        }
    });

    // Initialise on page load
    loadTheme();
    loadTasks();
})();

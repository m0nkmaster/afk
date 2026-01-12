// Task List Application

(function() {
    'use strict';

    // Constants
    var STORAGE_KEY = 'taskList';

    // DOM elements
    var taskForm = document.getElementById('task-form');
    var taskInput = document.getElementById('task-input');
    var taskList = document.getElementById('task-list');

    /**
     * Saves the current task list to localStorage
     */
    function saveTasks() {
        var tasks = [];
        var items = taskList.querySelectorAll('li');
        for (var i = 0; i < items.length; i++) {
            var item = items[i];
            tasks.push({
                text: item.querySelector('span').textContent,
                completed: item.classList.contains('completed')
            });
        }
        localStorage.setItem(STORAGE_KEY, JSON.stringify(tasks));
    }

    /**
     * Toggles the completed state of a task
     * @param {HTMLLIElement} taskElement - The task list item
     */
    function toggleComplete(taskElement) {
        taskElement.classList.toggle('completed');
        saveTasks();
    }

    /**
     * Deletes a task from the list
     * @param {HTMLLIElement} taskElement - The task list item to delete
     */
    function deleteTask(taskElement) {
        taskElement.remove();
        saveTasks();
    }

    /**
     * Creates a task list item element
     * @param {string} text - The task text
     * @returns {HTMLLIElement} The created list item
     */
    function createTaskElement(text) {
        var li = document.createElement('li');
        
        var span = document.createElement('span');
        span.textContent = text;
        span.addEventListener('click', function() {
            toggleComplete(li);
        });
        li.appendChild(span);
        
        var deleteBtn = document.createElement('button');
        deleteBtn.textContent = 'Delete';
        deleteBtn.setAttribute('aria-label', 'Delete task');
        deleteBtn.addEventListener('click', function() {
            deleteTask(li);
        });
        li.appendChild(deleteBtn);
        
        return li;
    }

    /**
     * Adds a new task to the list
     * @param {string} text - The task text
     */
    function addTask(text) {
        var taskElement = createTaskElement(text);
        taskList.appendChild(taskElement);
        saveTasks();
    }

    /**
     * Handles form submission to add a new task
     * @param {Event} event - The submit event
     */
    function handleSubmit(event) {
        event.preventDefault();
        
        var text = taskInput.value.trim();
        if (text) {
            addTask(text);
            taskInput.value = '';
            taskInput.focus();
        }
    }

    // Event listeners
    taskForm.addEventListener('submit', handleSubmit);
})();

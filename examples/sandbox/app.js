// Task List Application

const STORAGE_KEY = 'taskList';

document.addEventListener('DOMContentLoaded', () => {
    const taskForm = document.getElementById('task-form');
    const taskInput = document.getElementById('task-input');
    const taskList = document.getElementById('task-list');

    /**
     * Saves all tasks to localStorage
     */
    function saveTasks() {
        const tasks = [];
        const taskItems = taskList.querySelectorAll('.task-item');
        taskItems.forEach((item) => {
            const text = item.querySelector('.task-text').textContent;
            const completed = item.classList.contains('completed');
            tasks.push({ text, completed });
        });
        localStorage.setItem(STORAGE_KEY, JSON.stringify(tasks));
    }

    /**
     * Creates a task list item element
     * @param {string} text - The task description
     * @returns {HTMLLIElement} The task list item element
     */
    function createTaskElement(text) {
        const li = document.createElement('li');
        li.className = 'task-item';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';

        const taskText = document.createElement('span');
        taskText.className = 'task-text';
        taskText.textContent = text;

        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'delete-btn';
        deleteBtn.textContent = 'Delete';

        li.appendChild(checkbox);
        li.appendChild(taskText);
        li.appendChild(deleteBtn);

        return li;
    }

    /**
     * Toggles the completed state of a task
     * @param {HTMLLIElement} taskItem - The task list item element
     * @param {boolean} completed - Whether the task is completed
     */
    function toggleTaskComplete(taskItem, completed) {
        taskItem.classList.toggle('completed', completed);
    }

    /**
     * Deletes a task from the list
     * @param {HTMLLIElement} taskItem - The task list item element to delete
     */
    function deleteTask(taskItem) {
        taskItem.remove();
    }

    /**
     * Adds a new task to the list
     * @param {string} text - The task description
     */
    function addTask(text) {
        const taskElement = createTaskElement(text);
        
        // Add checkbox change handler for toggling completion
        const checkbox = taskElement.querySelector('input[type="checkbox"]');
        checkbox.addEventListener('change', () => {
            toggleTaskComplete(taskElement, checkbox.checked);
            saveTasks();
        });
        
        // Add delete button click handler
        const deleteBtn = taskElement.querySelector('.delete-btn');
        deleteBtn.addEventListener('click', () => {
            deleteTask(taskElement);
            saveTasks();
        });
        
        taskList.appendChild(taskElement);
    }

    // Handle form submission (add button click or Enter key)
    taskForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const text = taskInput.value.trim();
        if (text) {
            addTask(text);
            saveTasks();
            taskInput.value = '';
            taskInput.focus();
        }
    });
});

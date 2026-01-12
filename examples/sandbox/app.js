// Task List Application

document.addEventListener('DOMContentLoaded', () => {
    const taskForm = document.getElementById('task-form');
    const taskInput = document.getElementById('task-input');
    const taskList = document.getElementById('task-list');

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
     * Adds a new task to the list
     * @param {string} text - The task description
     */
    function addTask(text) {
        const taskElement = createTaskElement(text);
        taskList.appendChild(taskElement);
    }

    // Handle form submission (add button click or Enter key)
    taskForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const text = taskInput.value.trim();
        if (text) {
            addTask(text);
            taskInput.value = '';
            taskInput.focus();
        }
    });
});

console.log('Test site loaded successfully!');

function showMessage(message) {
    alert(message);
}

// Add some interactivity
document.addEventListener('DOMContentLoaded', function() {
    const title = document.querySelector('h1');
    title.addEventListener('click', function() {
        showMessage('Welcome to the test site!');
    });
});

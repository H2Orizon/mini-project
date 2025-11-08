document.addEventListener("DOMContentLoaded", function() {
    setTimeout(() =>{
        const alert = document.getElementById('flash-message');
        if (alert) {
            alert.classList.remove("show");
            alert.classList.add("hide")
            setTimeout(() => alert.remove(), 500)
        }
    }, 4000)
})
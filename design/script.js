document.querySelectorAll("[data-target]").forEach((button) => {
  button.addEventListener("click", () => {
    document.querySelectorAll("[data-target]").forEach((item) => item.classList.remove("active"));
    document.querySelectorAll(".direction").forEach((item) => item.classList.remove("active"));
    button.classList.add("active");
    document.getElementById(button.dataset.target).classList.add("active");
  });
});

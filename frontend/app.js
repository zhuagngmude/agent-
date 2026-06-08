const titles = window.AGENT_SWARM_NAV || {};

document.querySelectorAll("[data-page]").forEach((button) => {
  button.addEventListener("click", () => {
    const page = button.dataset.page;
    const title = titles[page];

    if (!title) return;

    document
      .querySelectorAll("[data-page]")
      .forEach((item) => item.classList.toggle("active", item === button));

    document
      .querySelectorAll(".page-view")
      .forEach((view) => view.classList.toggle("active", view.id === page));

    document.querySelector("#pageTitle").textContent = title[0];
    document.querySelector("#pageCrumb").textContent = title[1];
  });
});

import "@testing-library/jest-dom/vitest";

// jsdom does not implement the native dialog lifecycle. Browser focus/modal
// behavior is checked separately; tests still exercise open and cancel events.
HTMLDialogElement.prototype.showModal = function () {
  this.setAttribute("open", "");
};
HTMLDialogElement.prototype.close = function () {
  this.removeAttribute("open");
};

import { createGame, setNextDirection, step } from "./snake_logic.js";

const canvas = document.querySelector("[data-snake-canvas]");
const scoreEl = document.querySelector("[data-score]");
const statusEl = document.querySelector("[data-status]");
const restartBtn = document.querySelector("[data-restart]");
const pauseBtn = document.querySelector("[data-pause]");
const controls = document.querySelectorAll("[data-control]");

const ctx = canvas.getContext("2d");
const cellSize = 20;
const cols = 20;
const rows = 20;

canvas.width = cols * cellSize;
canvas.height = rows * cellSize;

let state = createGame({ cols, rows });
let lastTick = performance.now();
let accumulator = 0;
let isPaused = false;
const tickMs = 140;

function updateScore() {
  scoreEl.textContent = String(state.score);
}

function updateStatus(text) {
  statusEl.textContent = text;
}

function drawGrid() {
  ctx.strokeStyle = "#d4d4d4";
  ctx.lineWidth = 1;
  for (let x = 0; x <= cols; x += 1) {
    ctx.beginPath();
    ctx.moveTo(x * cellSize, 0);
    ctx.lineTo(x * cellSize, rows * cellSize);
    ctx.stroke();
  }
  for (let y = 0; y <= rows; y += 1) {
    ctx.beginPath();
    ctx.moveTo(0, y * cellSize);
    ctx.lineTo(cols * cellSize, y * cellSize);
    ctx.stroke();
  }
}

function drawSnake() {
  state.snake.forEach((segment, index) => {
    ctx.fillStyle = index === 0 ? "#1f2933" : "#3e4c59";
    ctx.fillRect(
      segment.x * cellSize + 1,
      segment.y * cellSize + 1,
      cellSize - 2,
      cellSize - 2
    );
  });
}

function drawFood() {
  if (!state.food) {
    return;
  }
  ctx.fillStyle = "#d64545";
  ctx.beginPath();
  ctx.arc(
    state.food.x * cellSize + cellSize / 2,
    state.food.y * cellSize + cellSize / 2,
    cellSize / 2 - 3,
    0,
    Math.PI * 2
  );
  ctx.fill();
}

function render() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#f5f5f5";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  drawGrid();
  drawFood();
  drawSnake();
}

function loop(now) {
  const delta = now - lastTick;
  lastTick = now;
  if (!isPaused) {
    accumulator += delta;
    while (accumulator >= tickMs) {
      const next = step(state);
      if (next !== state) {
        state = next;
        updateScore();
        if (state.isGameOver) {
          isPaused = true;
          updateStatus(state.isWin ? "You win. Restart?" : "Game over. Restart?");
          pauseBtn.textContent = "Resume";
        }
      }
      accumulator -= tickMs;
    }
  }
  render();
  requestAnimationFrame(loop);
}

function restart() {
  state = createGame({ cols, rows });
  isPaused = false;
  accumulator = 0;
  updateScore();
  updateStatus("Eat the food. Avoid walls and yourself.");
  pauseBtn.textContent = "Pause";
}

function togglePause() {
  if (state.isGameOver) {
    return;
  }
  isPaused = !isPaused;
  pauseBtn.textContent = isPaused ? "Resume" : "Pause";
  updateStatus(isPaused ? "Paused" : "Eat the food. Avoid walls and yourself.");
}

const keyToDir = {
  ArrowUp: { x: 0, y: -1 },
  ArrowDown: { x: 0, y: 1 },
  ArrowLeft: { x: -1, y: 0 },
  ArrowRight: { x: 1, y: 0 },
  w: { x: 0, y: -1 },
  s: { x: 0, y: 1 },
  a: { x: -1, y: 0 },
  d: { x: 1, y: 0 },
};

document.addEventListener("keydown", (event) => {
  if (event.key === " ") {
    event.preventDefault();
    togglePause();
    return;
  }
  if (event.key === "r" || event.key === "R") {
    restart();
    return;
  }
  const dir = keyToDir[event.key];
  if (dir) {
    event.preventDefault();
    state = setNextDirection(state, dir);
  }
});

restartBtn.addEventListener("click", restart);

pauseBtn.addEventListener("click", togglePause);

controls.forEach((btn) => {
  btn.addEventListener("click", () => {
    const dir = keyToDir[btn.dataset.control];
    if (dir) {
      state = setNextDirection(state, dir);
    }
  });
});

updateScore();
updateStatus("Eat the food. Avoid walls and yourself.");
requestAnimationFrame(loop);

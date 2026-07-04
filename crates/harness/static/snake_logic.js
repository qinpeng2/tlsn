export function createGame(options = {}) {
  const cols = options.cols ?? 20;
  const rows = options.rows ?? 20;
  const rng = options.rng ?? Math.random;

  const startX = Math.floor(cols / 2);
  const startY = Math.floor(rows / 2);
  const snake = [
    { x: startX, y: startY },
    { x: startX - 1, y: startY },
    { x: startX - 2, y: startY },
  ];

  const direction = { x: 1, y: 0 };

  const food = spawnFood(snake, cols, rows, rng);

  return {
    cols,
    rows,
    snake,
    direction,
    nextDirection: direction,
    food,
    score: 0,
    isGameOver: false,
    isWin: false,
  };
}

export function setNextDirection(state, dir) {
  if (!dir || typeof dir.x !== "number" || typeof dir.y !== "number") {
    return state;
  }

  const isOpposite = dir.x === -state.direction.x && dir.y === -state.direction.y;
  const canReverse = state.snake.length === 1;

  if (isOpposite && !canReverse) {
    return state;
  }

  return { ...state, nextDirection: { x: dir.x, y: dir.y } };
}

export function step(state, rng = Math.random) {
  if (state.isGameOver) {
    return state;
  }

  const direction = state.nextDirection ?? state.direction;
  const head = state.snake[0];
  const nextHead = { x: head.x + direction.x, y: head.y + direction.y };

  if (
    nextHead.x < 0 ||
    nextHead.y < 0 ||
    nextHead.x >= state.cols ||
    nextHead.y >= state.rows
  ) {
    return { ...state, direction, isGameOver: true };
  }

  const body = state.snake;
  for (let i = 0; i < body.length; i += 1) {
    if (body[i].x === nextHead.x && body[i].y === nextHead.y) {
      return { ...state, direction, isGameOver: true };
    }
  }

  const isEating =
    state.food && state.food.x === nextHead.x && state.food.y === nextHead.y;

  const newSnake = isEating
    ? [nextHead, ...state.snake]
    : [nextHead, ...state.snake.slice(0, -1)];

  let food = state.food;
  let score = state.score;
  let isWin = state.isWin;

  if (isEating) {
    score += 1;
    food = spawnFood(newSnake, state.cols, state.rows, rng);
    if (!food) {
      isWin = true;
      return {
        ...state,
        snake: newSnake,
        direction,
        nextDirection: direction,
        score,
        food,
        isGameOver: true,
        isWin,
      };
    }
  }

  return {
    ...state,
    snake: newSnake,
    direction,
    nextDirection: direction,
    score,
    food,
    isWin,
  };
}

export function spawnFood(snake, cols, rows, rng = Math.random) {
  const occupied = new Set(snake.map((segment) => `${segment.x},${segment.y}`));
  const openCells = [];

  for (let y = 0; y < rows; y += 1) {
    for (let x = 0; x < cols; x += 1) {
      const key = `${x},${y}`;
      if (!occupied.has(key)) {
        openCells.push({ x, y });
      }
    }
  }

  if (openCells.length === 0) {
    return null;
  }

  const index = Math.floor(rng() * openCells.length);
  return openCells[index];
}

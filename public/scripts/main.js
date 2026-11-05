class Main {
    constructor() {
        this.pageViewsKey = 'pageViewsCount';
        this.clickCount = 'clickCount';
        this.initializeCounter();
        this.initializeClickCounter();
        this.displayCount();
        this.displayClickCount();


        document.addEventListener("click", () => {
            this.incrementCount();
            this.displayClickCount();
        });
    }

    initializeCounter() {
        if (!localStorage.getItem(this.pageViewsKey)) {
            localStorage.setItem(this.pageViewsKey, '0');
        }
    }

    initializeClickCounter() {
        if (!localStorage.getItem(this.clickCount)) {
            localStorage.setItem(this.clickCount, '0');
        }    
    }

    incrementClickCount() {
        let currentCount = parseInt(localStorage.getItem(this.clickCount));
        currentCount++;
        localStorage.setItem(this.clickCount, currentCount.toString());
    }

    displayClickCount() {
        this.incrementClickCount();

        document.getElementById('clickCount').innerHTML = 'You have clicked ' + localStorage.getItem(this.clickCount)  + ' times on this page.'
    }
    
    incrementCount() {
        let currentCount = parseInt(localStorage.getItem(this.pageViewsKey));
        currentCount++;
        localStorage.setItem(this.pageViewsKey, currentCount.toString());
    }

    displayCount() {
        this.incrementCount();
        // Update count in div id count
        document.getElementById('count').innerHTML = 'You have visited this page ' + localStorage.getItem(this.pageViewsKey)  + ' times.'
    }
}

// Note that we construct the class here, but we don't need to assign it to a variable.
document.mainClass = new Main();

class CardPlayground {
    constructor(container) {
        this.container = container;
        this.cards = [...container.querySelectorAll('.page-section')];
        this.drag = null;
        this.layout();
        this.cards.forEach((card) => {
            card.addEventListener('pointerdown', (event) => this.grab(event, card));
            card.addEventListener('pointermove', (event) => this.move(event));
            card.addEventListener('pointerup', (event) => this.release(event));
            card.addEventListener('pointercancel', (event) => this.release(event, true));
            card.addEventListener('lostpointercapture', (event) => this.release(event, true));
        });
        // Re-measure the original responsive grid when its available width changes.
        this.observer = new ResizeObserver(() => {
            if (Math.abs(container.clientWidth - this.width) > 1) this.layout();
        });
        this.observer.observe(container);
        this.lastTime = null;
        requestAnimationFrame((time) => this.animate(time));
    }

    layout() {
        if (this.drag) this.release({ pointerId: this.drag.pointerId }, true);
        this.container.classList.remove('card-playground');
        this.container.style.height = '';
        this.cards.forEach((card) => {
            for (const property of ['width', 'height', 'left', 'top', 'transform']) {
                card.style[property] = '';
            }
        });
        const area = this.container.getBoundingClientRect();
        this.width = this.container.clientWidth;
        this.updateHorizontalBounds();
        this.bodies = this.cards.map((card) => {
            const rect = card.getBoundingClientRect();
            return { card, x: rect.left - area.left, y: rect.top - area.top,
                width: rect.width, height: rect.height, vx: 0, vy: 0 };
        });
        this.height = Math.max(...this.bodies.map((body) => body.y + body.height)) + 180;
        this.container.style.height = `${this.height}px`;
        this.container.classList.add('card-playground');
        this.bodies.forEach((body) => {
            Object.assign(body.card.style, {
                left: '0', top: '0', width: `${body.width}px`, height: `${body.height}px`
            });
        });
        this.render();
    }

    point(event) {
        const rect = this.container.getBoundingClientRect();
        return { x: event.clientX - rect.left, y: event.clientY - rect.top };
    }

    updateHorizontalBounds() {
        // Card positions use container coordinates; the walls use viewport edges.
        const left = this.container.getBoundingClientRect().left;
        this.minX = -left;
        this.maxX = document.documentElement.clientWidth - left;
    }

    grab(event, card) {
        if (this.drag || !event.isPrimary || event.button !== 0 ||
            event.target.closest('a, button, input, textarea, select, [contenteditable]')) return;
        const body = this.bodies.find((item) => item.card === card);
        const point = this.point(event);
        this.drag = { body, pointerId: event.pointerId,
            offsetX: point.x - body.x, offsetY: point.y - body.y,
            x: body.x, y: body.y };
        body.vx = body.vy = 0;
        card.classList.add('is-dragging');
        card.setPointerCapture(event.pointerId);
        event.preventDefault();
    }

    move(event) {
        if (!this.drag || event.pointerId !== this.drag.pointerId) return;
        const point = this.point(event);
        const { body } = this.drag;
        this.drag.x = Math.max(this.minX, Math.min(this.maxX - body.width, point.x - this.drag.offsetX));
        this.drag.y = Math.max(0, Math.min(this.height - body.height, point.y - this.drag.offsetY));
    }

    release(event, cancelled = false) {
        if (!this.drag || event.pointerId !== this.drag.pointerId) return;
        const { body, pointerId } = this.drag;
        this.drag = null;
        if (cancelled) body.vx = body.vy = 0;
        body.card.classList.remove('is-dragging');
        if (body.card.hasPointerCapture(pointerId)) body.card.releasePointerCapture(pointerId);
    }

    confine(body) {
        for (const [position, velocity, minimum, limit] of [
            ['x', 'vx', this.minX, this.maxX - body.width], ['y', 'vy', 0, this.height - body.height]
        ]) {
            if (body[position] < minimum) {
                body[position] = minimum;
                body[velocity] = Math.abs(body[velocity]) * 0.55;
            } else if (body[position] > limit) {
                body[position] = limit;
                body[velocity] = -Math.abs(body[velocity]) * 0.55;
            }
        }
    }

    collide(a, b) {
        const overlapX = Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x);
        const overlapY = Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y);
        if (overlapX <= 0 || overlapY <= 0) return;
        const horizontal = overlapX < overlapY;
        const position = horizontal ? 'x' : 'y';
        const velocity = horizontal ? 'vx' : 'vy';
        const size = horizontal ? 'width' : 'height';
        const normal = a[position] + a[size] / 2 < b[position] + b[size] / 2 ? 1 : -1;
        const depth = horizontal ? overlapX : overlapY;
        const massA = this.drag?.body === a ? 0.15 : 1;
        const massB = this.drag?.body === b ? 0.15 : 1;
        const total = massA + massB;
        a[position] -= normal * depth * massA / total;
        b[position] += normal * depth * massB / total;
        const approach = (b[velocity] - a[velocity]) * normal;
        if (approach < 0) {
            const impulse = -(1 + 0.65) * approach / total;
            a[velocity] -= impulse * massA * normal;
            b[velocity] += impulse * massB * normal;
        }
    }

    step(dt) {
        for (const body of this.bodies) {
            if (this.drag?.body === body) {
                // A damped spring follows the pointer and naturally supplies throw velocity.
                body.vx += ((this.drag.x - body.x) * 160 - body.vx * 22) * dt;
                body.vy += ((this.drag.y - body.y) * 160 - body.vy * 22) * dt;
            } else {
                const friction = Math.exp(-1.6 * dt);
                body.vx *= friction;
                body.vy *= friction;
                if (Math.hypot(body.vx, body.vy) < 2) body.vx = body.vy = 0;
            }
            const speed = Math.hypot(body.vx, body.vy);
            if (speed > 1800) {
                body.vx *= 1800 / speed;
                body.vy *= 1800 / speed;
            }
            body.x += body.vx * dt;
            body.y += body.vy * dt;
            this.confine(body);
        }
        // Multiple passes settle chains of cards against each other and the walls.
        for (let pass = 0; pass < 6; pass++) {
            for (let i = 0; i < this.bodies.length; i++) {
                for (let j = i + 1; j < this.bodies.length; j++) {
                    this.collide(this.bodies[i], this.bodies[j]);
                }
            }
            this.bodies.forEach((body) => this.confine(body));
        }
    }

    render() {
        this.bodies.forEach((body) => {
            body.card.style.transform = `translate(${body.x}px, ${body.y}px)`;
        });
    }

    animate(time) {
        this.updateHorizontalBounds();
        let remaining = this.lastTime === null ? 0 : Math.min((time - this.lastTime) / 1000, 0.05);
        this.lastTime = time;
        while (remaining > 0) {
            const dt = Math.min(remaining, 1 / 120);
            this.step(dt);
            remaining -= dt;
        }
        this.render();
        requestAnimationFrame((next) => this.animate(next));
    }
}

// Wait for the font metrics before measuring the cards.
document.fonts.ready.then(() => {
    document.cardPlayground = new CardPlayground(document.querySelector('.main-content'));
});

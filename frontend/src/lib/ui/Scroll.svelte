<script lang="ts">
	import type { Component } from '@lucide/svelte';
	import {
		shouldAutoScrollToBottom,
		shouldStickToBottomOnContentChange,
		type ScrollResetMode
	} from '$lib/ui/scroll';
	import { untrack } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		damping = 0.2,
		children,
		key = $bindable(null),
		resetTo = 'top',
		...props
	} = $props<
		HTMLAttributes<HTMLElement> & {
			children: Component;
			damping?: number;
			key?: any;
			resetTo?: ScrollResetMode;
		}
	>();

	let scrollElement = $state<null | HTMLDivElement>(null);
	let contentElement = $state<null | HTMLDivElement>(null);
	let velocity = $state(0);
	let animationFrameId: number | null = null;
	let isAnimating = $derived(animationFrameId != null);
	let isPinnedToBottom = $state(true);

	let lastKey = $state(key);

	function updatePinnedToBottom() {
		if (!scrollElement) return;
		isPinnedToBottom = shouldAutoScrollToBottom(scrollElement);
	}

	$effect(() => {
		if (!scrollElement) return;
		if (key !== lastKey) {
			untrack(() => (lastKey = key));
			if (resetTo === 'bottom' && !isPinnedToBottom) return;
			scrollElement.scrollTo({
				top: resetTo === 'bottom' ? scrollElement.scrollHeight : 0,
				behavior: 'instant'
			});
			updatePinnedToBottom();
		}
	});

	$effect(() => {
		if (!scrollElement) return;

		updatePinnedToBottom();

		const onScroll = () => updatePinnedToBottom();
		scrollElement.addEventListener('scroll', onScroll);

		return () => scrollElement?.removeEventListener('scroll', onScroll);
	});

	$effect(() => {
		if (!scrollElement || !contentElement || !window.ResizeObserver) return;

		const observer = new ResizeObserver(() => {
			if (!shouldStickToBottomOnContentChange(resetTo, isPinnedToBottom)) return;
			scrollElement?.scrollTo({
				top: scrollElement.scrollHeight,
				behavior: 'instant'
			});
			updatePinnedToBottom();
		});

		observer.observe(contentElement);
		return () => observer.disconnect();
	});

	const velocityThreshold = 3;

	function onwheel(event: WheelEvent) {
		if (scrollElement == null || scrollElement.getBoundingClientRect().right - 220 > event.pageX)
			return;
		const activeTage = document.activeElement?.tagName;
		if (activeTage == 'TEXTAREA' || activeTage == 'INPUT') return;

		if (Math.abs(event.deltaY / event.deltaX) < 1) return;
		event.preventDefault();

		if (animationFrameId !== null) {
			cancelAnimationFrame(animationFrameId);
			animationFrameId = null;
		}

		const impulse = event.deltaY * (1 - damping);
		velocity += impulse;

		if (!isAnimating) animate();
	}

	function animate() {
		if (!scrollElement) {
			if (animationFrameId !== null) {
				cancelAnimationFrame(animationFrameId);
				animationFrameId = null;
			}
			return;
		}

		velocity *= 1 - damping;

		scrollElement.scrollTop += Math.round(velocity);

		if (Math.abs(velocity) > velocityThreshold) {
			animationFrameId = requestAnimationFrame(animate);
		} else {
			isAnimating = false;
			velocity = 0;
			animationFrameId = null;
		}
	}
</script>

<div
	{...props}
	class="flex overflow-y-auto {props['class'] || ''}"
	style={props.style}
	bind:this={scrollElement}
	{onwheel}
>
	<div bind:this={contentElement} class="w-full min-w-0">
		{@render children()}
	</div>
</div>

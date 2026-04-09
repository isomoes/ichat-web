import { describe, expect, it } from 'vitest';
import { shouldAutoScrollToBottom, shouldStickToBottomOnContentChange } from './scroll';

describe('shouldAutoScrollToBottom', () => {
	it('returns true when the viewport is already near the bottom', () => {
		expect(
			shouldAutoScrollToBottom({
				scrollTop: 680,
				clientHeight: 300,
				scrollHeight: 1000
			})
		).toBe(true);
	});

	it('returns false when the user has scrolled away from the bottom', () => {
		expect(
			shouldAutoScrollToBottom({
				scrollTop: 200,
				clientHeight: 300,
				scrollHeight: 1000
			})
		).toBe(false);
	});
});

describe('shouldStickToBottomOnContentChange', () => {
	it('sticks to bottom while bottom anchoring is enabled and pinned', () => {
		expect(shouldStickToBottomOnContentChange('bottom', true)).toBe(true);
	});

	it('does not stick to bottom when the user has scrolled away', () => {
		expect(shouldStickToBottomOnContentChange('bottom', false)).toBe(false);
	});
});

// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

describe('BaseInput', () => {
    it('renders label linked to input when label prop is provided', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: { modelValue: '', label: 'Email', id: 'email-input' },
        })
        const label = wrapper.find('label')
        expect(label.exists()).toBe(true)
        expect(label.text()).toBe('Email')
        expect(label.attributes('for')).toBe('email-input')
        expect(wrapper.find('input').attributes('id')).toBe('email-input')
    })

    it('binds modelValue and emits update:modelValue on input', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: { modelValue: 'hello' },
        })
        const input = wrapper.find('input').element as HTMLInputElement
        expect(input.value).toBe('hello')

        await wrapper.find('input').setValue('world')
        expect(wrapper.emitted('update:modelValue')).toHaveLength(1)
        expect(wrapper.emitted('update:modelValue')![0]).toEqual(['world'])
    })

    it('displays error text and applies error styling', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: { modelValue: '', error: 'Invalid' },
        })
        expect(wrapper.text()).toContain('Invalid')
        const input = wrapper.find('input')
        expect(input.classes()).toContain('border-red-300')
        expect(input.classes()).toContain('text-red-900')
    })

    it('sets disabled attribute and styling when disabled', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: { modelValue: '', disabled: true },
        })
        const input = wrapper.find('input')
        expect(input.attributes('disabled')).toBeDefined()
        expect(input.classes()).toContain('bg-gray-100')
        expect(input.classes()).toContain('cursor-not-allowed')
    })

    it('forwards type, placeholder and required attributes', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: {
                modelValue: '',
                type: 'password',
                placeholder: 'Secret',
                required: true,
            },
        })
        const input = wrapper.find('input')
        expect(input.attributes('type')).toBe('password')
        expect(input.attributes('placeholder')).toBe('Secret')
        expect(input.attributes('required')).toBeDefined()
    })

    it('generates a fallback id when id prop is omitted', async () => {
        const BaseInput = (await import('./BaseInput.vue')).default
        const wrapper = mount(BaseInput, {
            props: { modelValue: '', label: 'Name' },
        })
        const inputId = wrapper.find('input').attributes('id')
        expect(inputId).toBeTruthy()
        expect(inputId).toContain('input-')
        expect(wrapper.find('label').attributes('for')).toBe(inputId)
        expect(wrapper.find('p').attributes('id')).toBe(`${inputId}-error`)
    })
})

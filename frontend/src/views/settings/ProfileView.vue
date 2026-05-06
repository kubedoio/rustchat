<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAuthStore } from '../../stores/auth';
import { Camera, Save, Trash2, ArrowLeft } from 'lucide-vue-next';
import RcAvatar from '../../components/ui/RcAvatar.vue';
import api from '../../api/client';
import {
    useThemeStore,
    THEME_OPTIONS,
    FONT_OPTIONS,
    FONT_SIZE_OPTIONS,
    type Theme,
    type ChatFont,
    type ChatFontSize,
} from '../../stores/theme';

const authStore = useAuthStore();
const themeStore = useThemeStore();
const user = computed(() => authStore.user);

const firstName = ref(user.value?.first_name || '');
const lastName = ref(user.value?.last_name || '');
const nickname = ref(user.value?.nickname || '');
const position = ref(user.value?.position || '');
const displayName = ref(user.value?.display_name || '');
const username = ref(user.value?.username || '');
const saving = ref(false);
const uploading = ref(false);
const error = ref<string | null>(null);
const success = ref(false);

const fileInput = ref<HTMLInputElement | null>(null);

const selectedTheme = computed(() => themeStore.theme);
const selectedFont = computed({
    get: () => themeStore.chatFont,
    set: (value: ChatFont) => themeStore.setChatFont(value),
});
const selectedFontSize = computed({
    get: () => themeStore.chatFontSize,
    set: (value: ChatFontSize) => themeStore.setChatFontSize(value),
});

const themes = THEME_OPTIONS;
const fonts = FONT_OPTIONS;
const fontSizes = FONT_SIZE_OPTIONS;
const fieldLabelClass = 'text-sm font-semibold text-text-2';
const fieldInputClass = 'w-full rounded-lg border border-border-1 bg-bg-surface-2 px-4 py-2.5 text-text-1 outline-none transition-standard placeholder:text-text-3 focus:border-brand focus:ring-2 focus:ring-brand/15';
const fieldHintClass = 'text-xs text-text-3';
const sectionCardClass = 'rounded-r-3 border border-border-1 bg-bg-surface-2 p-6 shadow-1';

function setTheme(theme: Theme) {
    themeStore.setTheme(theme);
}

function optionFontStyle(cssVar: string) {
    return { fontFamily: cssVar };
}

async function handleUpdateProfile() {
    if (!user.value) return;
    saving.value = true;
    error.value = null;
    success.value = false;
    try {
        // Use Mattermost-compatible patch endpoint
        await api.put('/users/me/patch', {
            first_name: firstName.value || undefined,
            last_name: lastName.value || undefined,
            nickname: nickname.value || undefined,
            position: position.value || undefined,
        }, {
            baseURL: '/api/v4',
        });
        // Also update username/display_name via our endpoint
        await api.put(`/users/${user.value.id}`, {
            display_name: displayName.value,
            username: username.value
        });
        await authStore.fetchMe();
        success.value = true;
        setTimeout(() => success.value = false, 3000);
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to update profile';
    } finally {
        saving.value = false;
    }
}

async function handleAvatarUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file || !user.value) return;

    uploading.value = true;
    error.value = null;
    
    const formData = new FormData();
    formData.append('image', file);

    try {
        await api.post(`/users/${user.value.id}/image`, formData, {
            baseURL: '/api/v4',
        });
        await authStore.fetchMe();
    } catch (e: any) {
        error.value = e.response?.data?.message || 'Failed to upload avatar';
    } finally {
        uploading.value = false;
        if (fileInput.value) fileInput.value.value = '';
    }
}

async function removeAvatar() {
    if (!user.value) return;
    try {
        await api.put(`/users/${user.value.id}`, { avatar_url: '' });
        await authStore.fetchMe();
    } catch (e) {
        console.error('Failed to remove avatar', e);
    }
}
</script>

<template>
    <div class="min-h-screen bg-bg-app p-6 text-text-1">
        <div class="mx-auto max-w-3xl space-y-8">
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-4">
                    <router-link to="/" class="rounded-full p-2 text-text-3 transition-colors hover:bg-bg-surface-2 hover:text-text-1">
                        <ArrowLeft class="w-6 h-6" />
                    </router-link>
                    <div>
                        <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-brand">Personal Studio</p>
                        <h1 class="text-[30px] font-semibold tracking-[-0.04em] text-text-1">Profile Settings</h1>
                    </div>
                </div>
            </div>

            <div class="overflow-hidden rounded-xl border border-border-1 bg-bg-surface-1 shadow-1">
                <div class="flex flex-col items-center border-b border-border-1 bg-[radial-gradient(circle_at_top,_color-mix(in_srgb,_var(--brand)_10%,transparent),transparent_55%)] p-8">
                    <div class="relative group">
                        <RcAvatar 
                            :userId="user?.id"
                            :username="user?.username"
                            :src="user?.avatar_url"
                            :size="120"
                            class="shadow-2 ring-4 ring-bg-surface-1"
                        />
                        <button 
                            @click="fileInput?.click()"
                            class="absolute inset-0 flex cursor-pointer items-center justify-center rounded-full bg-black/45 text-white opacity-0 transition-opacity group-hover:opacity-100"
                        >
                            <Camera class="w-8 h-8" />
                        </button>
                        <div v-if="uploading" class="absolute inset-0 z-10 flex items-center justify-center rounded-full bg-black/60 text-white">
                            <div class="h-8 w-8 animate-spin rounded-full border-2 border-white border-t-transparent"></div>
                        </div>
                    </div>
                    
                    <input 
                        ref="fileInput"
                        type="file" 
                        class="hidden" 
                        accept="image/*" 
                        @change="handleAvatarUpload"
                    />

                    <div class="mt-5 text-center">
                        <p class="text-lg font-semibold tracking-[-0.02em] text-text-1">{{ user?.display_name || user?.username }}</p>
                        <p class="text-sm text-text-3">@{{ user?.username }}</p>
                    </div>

                    <div class="mt-4 flex items-center space-x-4">
                        <button 
                            @click="fileInput?.click()"
                            class="text-sm font-medium text-brand transition-colors hover:text-brand-hover"
                        >
                            Change Photo
                        </button>
                        <button 
                            v-if="user?.avatar_url"
                            @click="removeAvatar"
                            class="flex items-center text-sm font-medium text-danger transition-colors hover:opacity-80"
                        >
                            <Trash2 class="mr-1 h-4 w-4" />
                            Remove
                        </button>
                    </div>
                </div>

                <div class="space-y-6 p-8">
                    <div v-if="error" class="rounded-lg border border-danger/20 bg-danger/10 p-4 text-sm text-danger">
                        {{ error }}
                    </div>
                    <div v-if="success" class="rounded-lg border border-success/20 bg-success/10 p-4 text-sm text-success">
                        Profile updated successfully!
                    </div>

                    <div class="grid grid-cols-1 gap-6">
                        <section :class="sectionCardClass">
                            <div class="mb-5 border-b border-border-1 pb-4">
                                <h2 class="text-base font-semibold text-text-1">Identity</h2>
                                <p class="mt-1 text-sm text-text-3">Update the personal details teammates see in channels, mentions, and profiles.</p>
                            </div>

                            <div class="grid grid-cols-1 gap-6">
                                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                                    <div class="space-y-2">
                                        <label :class="fieldLabelClass">First Name</label>
                                        <input
                                            v-model="firstName"
                                            type="text"
                                            placeholder="John"
                                            :class="fieldInputClass"
                                        />
                                    </div>
                                    <div class="space-y-2">
                                        <label :class="fieldLabelClass">Last Name</label>
                                        <input
                                            v-model="lastName"
                                            type="text"
                                            placeholder="Doe"
                                            :class="fieldInputClass"
                                        />
                                    </div>
                                </div>

                                <div class="space-y-2">
                                    <label :class="fieldLabelClass">Nickname</label>
                                    <input
                                        v-model="nickname"
                                        type="text"
                                        placeholder="Johnny"
                                        :class="fieldInputClass"
                                    />
                                    <p :class="fieldHintClass">How you want to be called.</p>
                                </div>

                                <div class="space-y-2">
                                    <label :class="fieldLabelClass">Position</label>
                                    <input
                                        v-model="position"
                                        type="text"
                                        placeholder="Software Engineer"
                                        :class="fieldInputClass"
                                    />
                                    <p :class="fieldHintClass">Your job title or role.</p>
                                </div>

                                <div class="space-y-2">
                                    <label :class="fieldLabelClass">Display Name</label>
                                    <input
                                        v-model="displayName"
                                        type="text"
                                        placeholder="e.g. John Doe"
                                        :class="fieldInputClass"
                                    />
                                    <p :class="fieldHintClass">This is how you'll appear to others in RustChat.</p>
                                </div>

                                <div class="space-y-2">
                                    <label :class="fieldLabelClass">Username</label>
                                    <div class="relative">
                                        <span class="absolute left-4 top-1/2 -translate-y-1/2 font-medium text-text-3">@</span>
                                        <input
                                            v-model="username"
                                            type="text"
                                            :class="`${fieldInputClass} pl-9 pr-4`"
                                        />
                                    </div>
                                </div>

                                <div class="space-y-2 border-t border-border-1 pt-4">
                                    <label :class="fieldLabelClass">Email Address</label>
                                    <input
                                        :value="user?.email"
                                        type="email"
                                        disabled
                                        class="w-full cursor-not-allowed rounded-lg border border-border-1 bg-bg-surface-1 px-4 py-2.5 text-text-3 opacity-80"
                                    />
                                    <p :class="fieldHintClass">Email address cannot be changed.</p>
                                </div>
                            </div>
                        </section>

                        <section :class="sectionCardClass">
                            <div class="mb-5 border-b border-border-1 pb-4">
                                <h2 class="text-base font-semibold text-text-1">Appearance</h2>
                                <p class="mt-1 text-sm text-text-3">Theme, typography, and sizing all apply live so you can check contrast as you go.</p>
                            </div>

                            <div class="space-y-2">
                                <p class="text-sm font-medium text-text-1">Theme Palette</p>
                                <p class="text-xs text-text-3">Pick one of 8 color themes.</p>
                                <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                                    <button
                                        v-for="theme in themes"
                                        :key="theme.id"
                                        type="button"
                                        @click="setTheme(theme.id)"
                                        class="rounded-lg border p-2 text-left transition-all"
                                        :class="selectedTheme === theme.id
                                            ? 'border-brand bg-brand/5 ring-2 ring-brand/20'
                                            : 'border-border-1 bg-bg-surface-1 hover:border-border-2 hover:bg-bg-surface-2'"
                                    >
                                        <div class="flex items-center gap-1.5">
                                            <span
                                                class="h-3.5 w-3.5 rounded-full border border-black/10"
                                                :style="{ backgroundColor: theme.swatches.primary }"
                                            ></span>
                                            <span
                                                class="h-3.5 w-3.5 rounded-full border border-black/10"
                                                :style="{ backgroundColor: theme.swatches.accent }"
                                            ></span>
                                            <span
                                                class="h-3.5 w-3.5 rounded-full border border-black/10"
                                                :style="{ backgroundColor: theme.swatches.background }"
                                            ></span>
                                        </div>
                                        <p class="mt-2 text-[11px] font-medium text-text-2">{{ theme.label }}</p>
                                    </button>
                                </div>
                            </div>
                        </section>

                        <section :class="sectionCardClass">
                            <div class="mb-5 border-b border-border-1 pb-4">
                                <h2 class="text-base font-semibold text-text-1">Typography</h2>
                                <p class="mt-1 text-sm text-text-3">Choose the reading tone and density that feels best for long chat sessions.</p>
                            </div>

                            <div class="space-y-5">
                                <div class="space-y-2">
                                    <label class="text-sm font-medium text-text-1">Font Preview</label>
                                    <select
                                        v-model="selectedFont"
                                        class="w-full rounded-lg border border-border-1 bg-bg-surface-1 px-3 py-2.5 text-text-1 outline-none transition-standard focus:border-brand focus:ring-2 focus:ring-brand/15"
                                    >
                                        <option
                                            v-for="font in fonts"
                                            :key="font.id"
                                            :value="font.id"
                                            :style="optionFontStyle(font.cssVar)"
                                        >
                                            {{ font.label }}
                                        </option>
                                    </select>
                                </div>

                                <div class="space-y-2">
                                    <label class="text-sm font-medium text-text-1">Chat Font Size</label>
                                    <div class="grid grid-cols-3 gap-2 sm:grid-cols-5">
                                        <label
                                            v-for="size in fontSizes"
                                            :key="size"
                                            class="cursor-pointer"
                                        >
                                            <input
                                                v-model="selectedFontSize"
                                                type="radio"
                                                class="sr-only"
                                                :value="size"
                                            />
                                            <div
                                                class="rounded-lg border px-2 py-2 text-center text-xs font-semibold transition-all"
                                                :class="selectedFontSize === size
                                                    ? 'border-brand bg-brand/10 text-brand'
                                                    : 'border-border-1 bg-bg-surface-1 text-text-2 hover:border-border-2'"
                                            >
                                                {{ size }}px
                                            </div>
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </section>
                    </div>

                    <div class="flex justify-end pt-6">
                        <button 
                            @click="handleUpdateProfile"
                            :disabled="saving"
                            class="flex items-center space-x-2 rounded-lg bg-brand px-8 py-2.5 font-bold text-brand-foreground shadow-lg shadow-brand/20 transition-all hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
                        >
                            <Save v-if="!saving" class="h-5 w-5" />
                            <div v-else class="h-5 w-5 animate-spin rounded-full border-2 border-current border-t-transparent"></div>
                            <span>{{ saving ? 'Saving...' : 'Save Changes' }}</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

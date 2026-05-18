import os
import re

mappings = {
    'messageStore': '@/features/messages/stores/messageStore',
    'unreadStore': '@/features/unreads/stores/unreadStore',
    'channelStore': '@/features/channels/stores/channelStore',
    'teamStore': '@/features/teams/stores/teamStore',
    'channelPreferencesStore': '@/features/channels/stores/channelPreferencesStore',
    # Handle direct store name imports that were moved
    'stores/messages': '@/features/messages/stores/messageStore',
    'stores/unreads': '@/features/unreads/stores/unreadStore',
    'stores/channels': '@/features/channels/stores/channelStore',
    'stores/teams': '@/features/teams/stores/teamStore',
}

# Regex to match imports and vi.mock
# Matches '...', "...", or `...` containing relative paths to the stores
patterns = [
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?channels/stores/channelStore(['\"])"), r"\1@/features/channels/stores/channelStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?messages/stores/messageStore(['\"])"), r"\1@/features/messages/stores/messageStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?teams/stores/teamStore(['\"])"), r"\1@/features/teams/stores/teamStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?unreads/stores/unreadStore(['\"])"), r"\1@/features/unreads/stores/unreadStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?channels/stores/channelPreferencesStore(['\"])"), r"\1@/features/channels/stores/channelPreferencesStore\2"),
    
    # Direct stores/ matches
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?stores/channels(['\"])"), r"\1@/features/channels/stores/channelStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?stores/messages(['\"])"), r"\1@/features/messages/stores/messageStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?stores/teams(['\"])"), r"\1@/features/teams/stores/teamStore\2"),
    (re.compile(r"(['\"])(?:\.\./)+(?:features/)?stores/unreads(['\"])"), r"\1@/features/unreads/stores/unreadStore\2"),

    # Special case for stores/auth.ts which has ./channels and ./teams
    (re.compile(r"(['\"])\./channels(['\"])"), r"\1@/features/channels/stores/channelStore\2"),
    (re.compile(r"(['\"])\./teams(['\"])"), r"\1@/features/teams/stores/teamStore\2"),
]

def update_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    new_content = content
    for pattern, replacement in patterns:
        new_content = pattern.sub(replacement, new_content)
    
    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for root, dirs, files in os.walk('frontend/src'):
    for file in files:
        if file.endswith(('.ts', '.vue', '.js')):
            update_file(os.path.join(root, file))

-- Playbooks Schema Migration
-- Workflow automation with checklists, runs, and tasks

-- Playbooks (templates for workflows)
CREATE TABLE IF NOT EXISTS playbooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id),
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    icon VARCHAR(10), -- Emoji icon
    
    -- Permissions
    is_public BOOLEAN DEFAULT false, -- Visible to all team members
    member_ids UUID[] DEFAULT '{}', -- Specific members who can use
    
    -- Settings
    create_channel_on_run BOOLEAN DEFAULT true,
    channel_name_template VARCHAR(100), -- e.g., "incident-{{date}}"
    default_owner_id UUID REFERENCES users(id),
    
    -- Triggers
    webhook_enabled BOOLEAN DEFAULT false,
    webhook_secret VARCHAR(64),
    keyword_triggers TEXT[], -- Keywords that trigger this playbook
    
    is_archived BOOLEAN DEFAULT false,
    version INTEGER DEFAULT 1,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Playbook checklists (groups of tasks)
CREATE TABLE IF NOT EXISTS playbook_checklists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    playbook_id UUID NOT NULL REFERENCES playbooks(id) ON DELETE CASCADE,
    
    name VARCHAR(255) NOT NULL,
    sort_order INTEGER DEFAULT 0,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Playbook tasks (items within checklists)
CREATE TABLE IF NOT EXISTS playbook_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    checklist_id UUID NOT NULL REFERENCES playbook_checklists(id) ON DELETE CASCADE,
    
    title VARCHAR(500) NOT NULL,
    description TEXT,
    
    -- Assignment
    default_assignee_id UUID REFERENCES users(id),
    assignee_role VARCHAR(50), -- e.g., 'owner', 'reporter'
    
    -- Timing
    due_after_minutes INTEGER, -- Due X minutes after run starts
    
    -- Automation
    slash_command VARCHAR(100), -- Slash command to run
    webhook_url TEXT, -- External webhook to call
    
    -- Conditional
    condition_attribute VARCHAR(100), -- e.g., 'severity'
    condition_value VARCHAR(100), -- e.g., 'high'
    
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Playbook runs (instances of playbook execution)
CREATE TABLE IF NOT EXISTS playbook_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    playbook_id UUID NOT NULL REFERENCES playbooks(id),
    team_id UUID NOT NULL REFERENCES teams(id),
    channel_id UUID REFERENCES channels(id), -- Associated channel
    
    name VARCHAR(255) NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id),
    
    -- Status
    status VARCHAR(50) DEFAULT 'in_progress', -- in_progress, finished, archived
    
    -- Attributes (custom key-value pairs)
    attributes JSONB DEFAULT '{}',
    
    -- Summary
    summary TEXT,
    
    -- Timing
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    finished_at TIMESTAMP WITH TIME ZONE,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Run task status (tracks completion of each task in a run)
CREATE TABLE IF NOT EXISTS run_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES playbook_runs(id) ON DELETE CASCADE,
    task_id UUID NOT NULL REFERENCES playbook_tasks(id),
    
    status VARCHAR(50) DEFAULT 'pending', -- pending, in_progress, done, skipped
    assignee_id UUID REFERENCES users(id),
    
    completed_at TIMESTAMP WITH TIME ZONE,
    completed_by UUID REFERENCES users(id),
    
    notes TEXT,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(run_id, task_id)
);

-- Run status updates (timeline of updates)
CREATE TABLE IF NOT EXISTS run_status_updates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES playbook_runs(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id),
    
    message TEXT NOT NULL,
    is_broadcast BOOLEAN DEFAULT false, -- Posted to stakeholder channels
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Run participants
CREATE TABLE IF NOT EXISTS run_participants (
    run_id UUID NOT NULL REFERENCES playbook_runs(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) DEFAULT 'participant', -- owner, participant, stakeholder
    
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY(run_id, user_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_playbooks_team ON playbooks(team_id);
CREATE INDEX IF NOT EXISTS idx_playbook_checklists_playbook ON playbook_checklists(playbook_id);
CREATE INDEX IF NOT EXISTS idx_playbook_tasks_checklist ON playbook_tasks(checklist_id);
CREATE INDEX IF NOT EXISTS idx_playbook_runs_playbook ON playbook_runs(playbook_id);
CREATE INDEX IF NOT EXISTS idx_playbook_runs_team ON playbook_runs(team_id);
CREATE INDEX IF NOT EXISTS idx_playbook_runs_status ON playbook_runs(status);
CREATE INDEX IF NOT EXISTS idx_run_tasks_run ON run_tasks(run_id);
CREATE INDEX IF NOT EXISTS idx_run_status_updates_run ON run_status_updates(run_id);

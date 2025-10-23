-- Add Kamino and Drift to program registry

INSERT INTO program_registry (program_id, program_name, program_type, description) VALUES
    ('KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD', 'Kamino Lend', 'Lending', 'Kamino lending protocol'),
    ('dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH', 'Drift Protocol', 'Derivatives', 'Drift perpetuals and derivatives exchange')
ON CONFLICT (program_id) DO NOTHING;

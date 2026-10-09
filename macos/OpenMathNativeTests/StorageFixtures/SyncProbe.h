#ifndef OM_SYNC_PROBE_H
#define OM_SYNC_PROBE_H
int om_fixture_install_sync_probe(void);
int om_fixture_fullsync_calls(void);
int om_fixture_fullsync_successes(void);
int om_fixture_barrier_calls(void);
int om_fixture_unknown_fcntl_calls(void);
void om_fixture_fail_fullsync(int enabled);
void om_fixture_fail_writes(int enabled);
#endif

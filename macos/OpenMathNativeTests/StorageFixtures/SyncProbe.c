/* Test-only public SQLite VFS syscall hooks. All successful calls delegate to the real OS. */
#include "SyncProbe.h"
#include <sqlite3.h>
#include <fcntl.h>
#include <errno.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
static int (*real_fcntl)(int,int,...);
static ssize_t (*real_write)(int,const void*,size_t);
static ssize_t (*real_pwrite)(int,const void*,size_t,off_t);
static atomic_int calls,successes,barriers,unknown,fail_writes,fail_fullsync;
static int traced_fcntl(int fd,int command,...) {
  if(command==F_FULLFSYNC
#ifdef F_BARRIERFSYNC
     ||command==F_BARRIERFSYNC
#endif
  ) {
    return real_fcntl(fd,command);
  }
  if(command==F_GETFD || command==F_GETFL) return real_fcntl(fd,command);
  va_list args;va_start(args,command);int result;
  if(command==F_SETLK || command==F_SETLKW || command==F_GETLK || command==F_OFD_SETLK || command==F_OFD_SETLKW || command==F_OFD_GETLK || command==F_OFD_SETLKWTIMEOUT) {
    void *argument=va_arg(args,void*);result=real_fcntl(fd,command,argument);
  } else if(command==F_SETFD || command==F_SETFL || command==F_DUPFD) {
    int argument=va_arg(args,int);result=real_fcntl(fd,command,argument);
  } else {
    fprintf(stderr,"unknown SQLite fcntl command: %d\n",command);atomic_fetch_add(&unknown,1);errno=EINVAL;result=-1;
  }
  va_end(args);return result;
}
static ssize_t traced_write(int fd,const void *bytes,size_t count) {
  if(atomic_load(&fail_writes)){errno=ENOSPC;return -1;}
  return real_write(fd,bytes,count);
}
static ssize_t traced_pwrite(int fd,const void *bytes,size_t count,off_t offset) {
  if(atomic_load(&fail_writes)){errno=ENOSPC;return -1;}
  return real_pwrite(fd,bytes,count,offset);
}
int om_fixture_install_sync_probe(void) {
  sqlite3_vfs *vfs=sqlite3_vfs_find(0);
  if(!vfs || vfs->iVersion<3 || !vfs->xGetSystemCall || !vfs->xSetSystemCall)return SQLITE_NOTFOUND;
  real_fcntl=(int(*)(int,int,...))vfs->xGetSystemCall(vfs,"fcntl");
  real_write=(ssize_t(*)(int,const void*,size_t))vfs->xGetSystemCall(vfs,"write");
  real_pwrite=(ssize_t(*)(int,const void*,size_t,off_t))vfs->xGetSystemCall(vfs,"pwrite");
  if(!real_fcntl || !real_write || !real_pwrite)return SQLITE_NOTFOUND;
  int result=vfs->xSetSystemCall(vfs,"fcntl",(sqlite3_syscall_ptr)traced_fcntl);
  if(result==SQLITE_OK)result=vfs->xSetSystemCall(vfs,"write",(sqlite3_syscall_ptr)traced_write);
  if(result==SQLITE_OK)result=vfs->xSetSystemCall(vfs,"pwrite",(sqlite3_syscall_ptr)traced_pwrite);
  return result;
}
int om_fixture_fullsync_calls(void){return atomic_load(&calls);}
int om_fixture_fullsync_successes(void){return atomic_load(&successes);}
int om_fixture_barrier_calls(void){return atomic_load(&barriers);}
int om_fixture_unknown_fcntl_calls(void){return atomic_load(&unknown);}
void om_fixture_fail_fullsync(int enabled){atomic_store(&fail_fullsync,enabled);}
void om_fixture_fail_writes(int enabled){atomic_store(&fail_writes,enabled);}

/* Test-process interposition of SDK-exported public fcntl entry points. Only SQLite FDs
 * are counted, all successful calls reach the real OS. Never linked into the app. */
extern int original_fcntl_nocancel(int,int,...) __asm("_fcntl$NOCANCEL");
static int record_public_call(int (*original)(int,int,...),int fd,int command,va_list args) {
  int sync=command==F_FULLFSYNC;
#ifdef F_BARRIERFSYNC
  sync=sync || command==F_BARRIERFSYNC;
#endif
  int tracked=0;
  if(sync) {
    char path[4096]={0};
    if(fcntl(fd,F_GETPATH,path)==0 && strstr(path,".sqlite"))tracked=1;
  }
  if(tracked && command==F_FULLFSYNC && atomic_load(&fail_fullsync)){errno=EIO;return -1;}
  int result;
  if(sync || command==F_GETFD || command==F_GETFL || command==F_GETPROTECTIONCLASS) result=original(fd,command);
  else if(command==F_SETLK || command==F_SETLKW || command==F_GETLK || command==F_OFD_SETLK || command==F_OFD_SETLKW || command==F_OFD_GETLK || command==F_OFD_SETLKWTIMEOUT || command==F_GETPATH )result=original(fd,command,va_arg(args,void*));
  /* Observed SDK command 95 is F_SETCONFINED; Apple fcntl-base forwards an int.
   * This test merely forwards SQLite's existing call, and never issues it from the app. */
  else if(command==F_SETFD || command==F_SETFL || command==F_DUPFD || command==F_NOCACHE || command==F_RDAHEAD || command==F_GETPROTECTIONLEVEL || command==95)result=original(fd,command,va_arg(args,int));
  else {fprintf(stderr,"unknown public fcntl command: %d\n",command);atomic_fetch_add(&unknown,1);errno=EINVAL;result=-1;}
  if(tracked){
#ifdef F_BARRIERFSYNC
 if(command==F_BARRIERFSYNC)atomic_fetch_add(&barriers,1);else
#endif
 {atomic_fetch_add(&calls,1);if(result==0)atomic_fetch_add(&successes,1);}
 }
  return result;
}
static int traced_public_fcntl(int fd,int command,...) {
  va_list args;va_start(args,command);int result=record_public_call(fcntl,fd,command,args);va_end(args);return result;
}
static int traced_public_nocancel(int fd,int command,...) {
  va_list args;va_start(args,command);int result=record_public_call(original_fcntl_nocancel,fd,command,args);va_end(args);return result;
}
__attribute__((used)) static struct { const void *replacement;const void *original; } public_sync_interpose[]
__attribute__((section("__DATA,__interpose"))) = {
{(const void*)traced_public_fcntl,(const void*)fcntl},
{(const void*)traced_public_nocancel,(const void*)original_fcntl_nocancel}};

//! zstd's `--rsyncable` option performs content defined chunking
//!
//! It _appears_ distinct from the chunking method used in 

/*
 *  findSynchronizationPoint()
 *  https://github.com/facebook/zstd/blob/15c5e200235edc520c1bd678ed126a6dd05736e1/lib/compress/zstdmt_compress.c#L1931-L2001
 *
 *
  if (params.rsyncable) {
        /* Aim for the targetsectionSize as the average job size. */
        U32 const jobSizeMB = (U32)(mtctx->targetSectionSize >> 20);
        U32 const rsyncBits = ZSTD_highbit32(jobSizeMB) + 20;
        assert(jobSizeMB >= 1);
        DEBUGLOG(4, "rsyncLog = %u", rsyncBits);
        mtctx->rsync.hash = 0;
        mtctx->rsync.hitMask = (1ULL << rsyncBits) - 1;
        mtctx->rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH);
    }
 */



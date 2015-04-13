// C header library matching rust-gzip

#ifndef __RGZIP_H
#define __RGZIP_H

void * decompress_gzip_to_heap(const void * buf,
    int buf_len,
    int * new_buf_len);


#endif

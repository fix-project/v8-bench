#pragma once

#include <sys/eventfd.h>
#include <unistd.h>

#include <cerrno>
#include <cstdint>
#include <string>

#include "exception.hh"
#include "file_descriptor.hh"

class EventFD : public FileDescriptor
{
public:
  EventFD( const bool semaphore = false )
    : FileDescriptor( eventfd( 0u, ( semaphore ? EFD_SEMAPHORE : 0 ) | EFD_NONBLOCK ) )
  {}

  EventFD( FileDescriptor&& fd )
    : FileDescriptor( std::move( fd ) )
  {}

  bool read_event()
  {
    uint64_t value;
    int retval = ::read( fd_num(), &value, sizeof( value ) );

    register_read();

    if ( retval == sizeof( value ) ) {
      return true;
    } else if ( retval < 0 && errno == EAGAIN ) {
      return false;
    } else {
      throw unix_error( "eventfd_read" );
    }
  }

  void write_event()
  {
    uint64_t value = 1;
    if ( ::write( fd_num(), &value, sizeof( value ) ) < 0 ) {
      throw unix_error( "eventfd_write" );
    }

    register_write();
  }

  EventFD duplicate() const { return EventFD( FileDescriptor::duplicate() ); }
};
